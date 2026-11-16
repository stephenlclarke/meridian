//! Original ScriptWire tags, integer widths, sorted keys and bounds.
use crate::Error;
use std::collections::BTreeMap;

pub const MAX_BYTES: usize = 1_048_576;
pub const MAX_ITEMS: usize = 16_384;
pub const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Map(BTreeMap<String, Value>),
}
impl Value {
    pub fn map(entries: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Self {
        Self::Map(entries.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
    pub fn empty_map() -> Self {
        Self::Map(BTreeMap::new())
    }
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(map) => map.get(key),
            _ => None,
        }
    }
    pub fn string(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
    pub fn integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }
    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Map(value) => Some(value),
            _ => None,
        }
    }
}

pub fn encode(values: &[Value], limit: usize) -> Result<Vec<u8>, Error> {
    encode_refs(&values.iter().collect::<Vec<_>>(), limit)
}

pub(crate) fn encode_refs(values: &[&Value], limit: usize) -> Result<Vec<u8>, Error> {
    if limit > MAX_BYTES {
        return Err(Error::Limit("Script data exceeds its byte budget."));
    }
    let mut writer = Writer {
        bytes: Vec::new(),
        items: 0,
        limit,
    };
    for value in values {
        writer.put(value, 0)?;
    }
    Ok(writer.bytes)
}
struct Writer {
    bytes: Vec<u8>,
    items: usize,
    limit: usize,
}
impl Writer {
    fn bytes(&mut self, value: &[u8]) -> Result<(), Error> {
        if value.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(Error::Limit("Script data exceeds its byte budget."));
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }
    fn length(&mut self, value: usize) -> Result<(), Error> {
        let value = u32::try_from(value).map_err(|_| Error::Limit("wire length"))?;
        self.bytes(&value.to_le_bytes())
    }
    fn string(&mut self, value: &str) -> Result<(), Error> {
        if value.len() > self.limit {
            return Err(Error::Limit("Script string is too large."));
        }
        self.length(value.len())?;
        self.bytes(value.as_bytes())
    }
    fn put(&mut self, value: &Value, depth: usize) -> Result<(), Error> {
        self.items += 1;
        if depth >= MAX_DEPTH || self.items > MAX_ITEMS {
            return Err(Error::Limit("Script data is too deeply nested or large."));
        }
        match value {
            Value::Null => self.bytes(&[0]),
            Value::Bool(value) => self.bytes(&[if *value { 2 } else { 1 }]),
            Value::Integer(value) => {
                self.bytes(&[3])?;
                self.bytes(&value.to_le_bytes())
            }
            Value::Number(value) => {
                if !value.is_finite() {
                    return Err(Error::Codec("Script numbers must be finite."));
                }
                self.bytes(&[4])?;
                self.bytes(&value.to_bits().to_le_bytes())
            }
            Value::String(value) => {
                self.bytes(&[5])?;
                self.string(value)
            }
            Value::Array(values) => {
                if values.len() > MAX_ITEMS {
                    return Err(Error::Limit("Script collection is too large."));
                }
                self.bytes(&[6])?;
                self.length(values.len())?;
                for value in values {
                    self.put(value, depth + 1)?;
                }
                Ok(())
            }
            Value::Map(values) => {
                if values.len() > MAX_ITEMS {
                    return Err(Error::Limit("Script collection is too large."));
                }
                self.bytes(&[7])?;
                self.length(values.len())?;
                for (key, value) in values {
                    self.string(key)?;
                    self.put(value, depth + 1)?;
                }
                Ok(())
            }
        }
    }
}

pub fn decode(bytes: &[u8]) -> Result<Value, Error> {
    if bytes.len() > MAX_BYTES {
        return Err(Error::Limit("Script output is too large."));
    }
    let mut reader = Reader {
        bytes,
        cursor: 0,
        items: 0,
    };
    let value = reader.get(0)?;
    if reader.cursor != bytes.len() {
        return Err(Error::Codec("Trailing script result bytes."));
    }
    Ok(value)
}
struct Reader<'a> {
    bytes: &'a [u8],
    cursor: usize,
    items: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        if count > self.bytes.len().saturating_sub(self.cursor) {
            return Err(Error::Codec("Truncated script result."));
        }
        let start = self.cursor;
        self.cursor += count;
        Ok(&self.bytes[start..self.cursor])
    }
    fn fixed<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        self.take(N)?
            .try_into()
            .map_err(|_| Error::Codec("fixed-width value"))
    }
    fn length(&mut self) -> Result<usize, Error> {
        Ok(u32::from_le_bytes(self.fixed()?) as usize)
    }
    fn string(&mut self) -> Result<String, Error> {
        let count = self.length()?;
        let bytes = self.take(count)?;
        String::from_utf8(bytes.to_vec()).map_err(|_| Error::Codec("Script strings must be UTF-8."))
    }
    fn get(&mut self, depth: usize) -> Result<Value, Error> {
        self.items += 1;
        if depth >= MAX_DEPTH || self.items > MAX_ITEMS {
            return Err(Error::Limit("Script result nesting/items exceeded."));
        }
        let [tag] = self.fixed()?;
        match tag {
            0 => Ok(Value::Null),
            1 | 2 => Ok(Value::Bool(tag == 2)),
            3 => Ok(Value::Integer(i64::from_le_bytes(self.fixed()?))),
            4 => {
                let value = f64::from_bits(u64::from_le_bytes(self.fixed()?));
                if !value.is_finite() {
                    return Err(Error::Codec("Nonfinite script result."));
                }
                Ok(Value::Number(value))
            }
            5 => Ok(Value::String(self.string()?)),
            6 | 7 => {
                let count = self.length()?;
                if count > MAX_ITEMS {
                    return Err(Error::Limit("Script result collection exceeded."));
                }
                if tag == 6 {
                    let mut values = Vec::new();
                    for _ in 0..count {
                        values.push(self.get(depth + 1)?);
                    }
                    Ok(Value::Array(values))
                } else {
                    let mut values = BTreeMap::new();
                    for _ in 0..count {
                        let key = self.string()?;
                        if values.contains_key(&key) {
                            return Err(Error::Codec("Duplicate script result key."));
                        }
                        values.insert(key, self.get(depth + 1)?);
                    }
                    Ok(Value::Map(values))
                }
            }
            _ => Err(Error::Codec("Invalid script result tag.")),
        }
    }
}
