//! Exact original typed value shape, distinct from untyped JSON and native World save loading.
use crate::{Error, Value};
use serde_json::Value as Json;
use std::collections::BTreeMap;

pub fn decode_tagged(value: &Json) -> Result<Value, Error> {
    get(value, 0, &mut Budget { items: 0, bytes: 0 })
}
struct Budget {
    items: usize,
    bytes: usize,
}
impl Budget {
    fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        if bytes > 262_144_usize.saturating_sub(self.bytes) {
            return Err(Error::Limit("Saved script value byte limit exceeded."));
        }
        self.bytes += bytes;
        Ok(())
    }
}
fn get(value: &Json, depth: usize, budget: &mut Budget) -> Result<Value, Error> {
    budget.items += 1;
    if depth >= 32 || budget.items > 16_384 {
        return Err(Error::Limit(
            "Saved script value depth or count limit exceeded.",
        ));
    }
    let fields = value
        .as_object()
        .ok_or(Error::Codec("Invalid saved script value fields."))?;
    let kind = fields
        .get("type")
        .and_then(Json::as_str)
        .ok_or(Error::Codec("Invalid saved script value fields."))?;
    if fields.len() != if kind == "null" { 1 } else { 2 }
        || (kind != "null" && !fields.contains_key("value"))
    {
        return Err(Error::Codec("Invalid saved script value fields."));
    }
    let raw = fields.get("value").unwrap_or(&Json::Null);
    match kind {
        "null" => {
            budget.charge(1)?;
            Ok(Value::Null)
        }
        "bool" => {
            budget.charge(1)?;
            Ok(Value::Bool(
                raw.as_bool()
                    .ok_or(Error::Codec("Saved script bool type."))?,
            ))
        }
        "integer" => {
            budget.charge(9)?;
            Ok(Value::Integer(
                raw.as_i64()
                    .ok_or(Error::Codec("Saved script integer type."))?,
            ))
        }
        "number" => {
            let text = raw
                .as_str()
                .ok_or(Error::Codec("Invalid saved floating bits."))?;
            if text.len() != 16
                || !text
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(Error::Codec("Invalid saved floating bits."));
            }
            let bits = u64::from_str_radix(text, 16)
                .map_err(|_| Error::Codec("Invalid saved floating bits."))?;
            let number = f64::from_bits(bits);
            if !number.is_finite() {
                return Err(Error::Codec("Saved script numbers must be finite."));
            }
            budget.charge(9)?;
            Ok(Value::Number(number))
        }
        "string" => {
            let text = raw
                .as_str()
                .ok_or(Error::Codec("Saved script string type."))?;
            budget.charge(5 + text.len())?;
            Ok(Value::String(text.to_owned()))
        }
        "array" => {
            let rows = raw
                .as_array()
                .ok_or(Error::Codec("Saved script array type."))?;
            if rows.len() > 16_384 {
                return Err(Error::Limit("Saved script collection limit exceeded."));
            }
            budget.charge(5)?;
            rows.iter()
                .map(|v| get(v, depth + 1, budget))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        "object" => {
            let rows = raw
                .as_object()
                .ok_or(Error::Codec("Saved script object type."))?;
            if rows.len() > 16_384 {
                return Err(Error::Limit("Saved script collection limit exceeded."));
            }
            budget.charge(5)?;
            let mut map = BTreeMap::new();
            for (key, value) in rows {
                budget.charge(4 + key.len())?;
                map.insert(key.clone(), get(value, depth + 1, budget)?);
            }
            Ok(Value::Map(map))
        }
        _ => Err(Error::Codec("Unknown saved script value type.")),
    }
}
