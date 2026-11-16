//! One protected Lua 5.5 VM per callback. No host gameplay callbacks enter a Lua frame.
use crate::value::{MAX_BYTES, MAX_DEPTH, MAX_ITEMS};
use crate::{Error, LuaFailureStage, Value, encode};
use mlua::{ChunkMode, Function, HookTriggers, Lua, LuaOptions, StdLib, Table, VmState};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub instructions: u64,
    pub heap_bytes: usize,
    pub allocation_bytes: usize,
    pub output_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            instructions: 50_000,
            heap_bytes: 8_388_608,
            allocation_bytes: 33_554_432,
            output_bytes: 524_288,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<(), Error> {
        // ScriptEvaluator's Swift preflight precedes the native C input-range checks.
        if self.heap_bytes < 65_536
            || self.allocation_bytes < self.heap_bytes
            || self.output_bytes == 0
        {
            return Err(Error::Pack("Invalid Lua handler or execution limits."));
        }
        if !(1_000..=1_000_000).contains(&self.instructions)
            || !(65_536..=16_777_216).contains(&self.heap_bytes)
            || !(self.heap_bytes..=67_108_864).contains(&self.allocation_bytes)
            || !(1..=MAX_BYTES).contains(&self.output_bytes)
        {
            return Err(Error::Limit("Invalid script input or limits."));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Evaluation {
    pub data: Value,
    pub commands: Vec<Value>,
    pub instructions: u64,
    pub peak_heap_bytes: usize,
    pub allocation_bytes: usize,
}

pub(crate) fn handler_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    (1..=80).contains(&name.len())
        && bytes
            .next()
            .is_some_and(|b| b.is_ascii_alphabetic() || b == b'_')
        && bytes.all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn evaluate(
    source: &[u8],
    handler: &str,
    event: &Value,
    snapshot: &Value,
    state: &Value,
    limits: Limits,
) -> Result<Evaluation, Error> {
    if !handler_name(handler) {
        return Err(Error::Pack("Invalid Lua handler or execution limits."));
    }
    crate::value::encode_refs(&[event, snapshot, state], MAX_BYTES)?;
    let (output, instructions, memory) = invoke(source, limits, |lua, module| {
        let function: Function = module
            .raw_get(handler)
            .map_err(|_| Error::Pack("Missing named handler."))?;
        let arguments = (
            to_lua(lua, event)?,
            to_lua(lua, snapshot)?,
            to_lua(lua, state)?,
        );
        let result = function
            .call::<mlua::Value>(arguments)
            .map_err(|cause| lua_failure(LuaFailureStage::Execution, cause))?;
        let mut budget = OutputBudget {
            items: 0,
            bytes: 0,
            limit: limits.output_bytes,
            ancestors: Vec::new(),
        };
        from_lua(result, 0, &mut budget)
    })?;
    let mut fields = match output {
        Value::Map(fields)
            if fields.len() == 2
                && fields.contains_key("data")
                && fields.contains_key("commands") =>
        {
            fields
        }
        _ => return Err(shape_error()),
    };
    let data = fields.remove("data").ok_or_else(shape_error)?;
    if !matches!(data, Value::Map(_)) {
        return Err(shape_error());
    }
    let commands = match fields.remove("commands").ok_or_else(shape_error)? {
        Value::Array(values) if values.len() <= 256 => values,
        Value::Map(values) if values.is_empty() => Vec::new(),
        _ => return Err(shape_error()),
    };
    encode(std::slice::from_ref(&data), 262_144)?;
    Ok(Evaluation {
        data,
        commands,
        instructions,
        peak_heap_bytes: memory.peak_heap_bytes(),
        allocation_bytes: memory.allocated_bytes(),
    })
}
fn shape_error() -> Error {
    Error::Pack("Handler must return {data = a map, commands = an array of at most 256 commands}.")
}

pub fn validate(source: &[u8], handlers: &[String]) -> Result<(), Error> {
    if handlers.len() > 64 {
        return Err(Error::Pack("Too many declared handlers."));
    }
    invoke(source, Limits::default(), |_lua, module| {
        for handler in handlers {
            if !matches!(
                module.raw_get::<mlua::Value>(handler.as_str())?,
                mlua::Value::Function(_)
            ) {
                return Err(Error::Lua(format!("Missing declared handler: {handler}")));
            }
        }
        Ok(())
    })?;
    Ok(())
}

fn invoke<T>(
    source: &[u8],
    limits: Limits,
    call: impl FnOnce(&Lua, Table) -> Result<T, Error>,
) -> Result<(T, u64, Arc<mlua::AllocationBudget>), Error> {
    limits.validate()?;
    if source.len() > 262_144 {
        return Err(Error::Limit("Invalid script input or limits."));
    }
    let memory = Arc::new(mlua::AllocationBudget::new(limits.allocation_bytes));
    let mut options = LuaOptions::new();
    options.hash_seed = Some(1); // Exact original lua_newstate hash seed, not an OS/wall-clock seed.
    options.allocation_budget = Some(memory.clone());
    let lua = Lua::new_with(StdLib::MATH, options)?;
    if lua.used_memory() > limits.heap_bytes || memory.allocated_bytes() > limits.allocation_bytes {
        return Err(Error::Limit("Cannot allocate interpreter."));
    }
    lua.set_memory_limit(limits.heap_bytes)?;
    let instructions = Rc::new(Cell::new(0_u64));
    let count = instructions.clone();
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(1000),
        move |_, _| {
            count.set(count.get() + 1000);
            if count.get() >= limits.instructions {
                return Err(mlua::Error::RuntimeError(
                    "Instruction budget exceeded.".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    let globals = lua.globals();
    for name in [
        "_G",
        "collectgarbage",
        "dofile",
        "loadfile",
        "load",
        "print",
        "warn",
        "pcall",
        "xpcall",
        "getmetatable",
        "setmetatable",
        "rawget",
        "rawset",
        "rawequal",
        "pairs",
        "next",
    ] {
        globals.raw_set(name, mlua::Nil)?;
    }
    let math: Table = globals.raw_get("math")?;
    math.raw_set("random", mlua::Nil)?;
    math.raw_set("randomseed", mlua::Nil)?;
    globals.raw_set(
        "tostring",
        lua.create_function(|lua, value: mlua::Value| match value {
            mlua::Value::Nil => lua.create_string("nil"),
            mlua::Value::Boolean(value) => lua.create_string(if value { "true" } else { "false" }),
            mlua::Value::String(_) | mlua::Value::Integer(_) | mlua::Value::Number(_) => {
                let value = lua.coerce_string(value)?.ok_or_else(|| {
                    mlua::Error::RuntimeError("Scalar string conversion failed.".into())
                })?;
                Ok(value)
            }
            _ => Err(mlua::Error::RuntimeError(
                "tostring accepts scalar values only.".into(),
            )),
        })?,
    )?;
    // A scoped original error function cannot retain a VM through a Rust callback cycle.
    let original_error: Function = globals.raw_get("error")?;
    let original_assert: Function = globals.raw_get("assert")?;
    let output = lua
        .scope(|scope| {
            let error =
                scope.create_function(move |_, (value, level): (mlua::Value, Option<i32>)| {
                    if !matches!(value, mlua::Value::String(_)) {
                        return Err(mlua::Error::external(Error::Codec(
                            "Script execution failed.",
                        )));
                    }
                    original_error.call::<()>((value, level))
                })?;
            globals.raw_set("error", error)?;
            let assertion = scope.create_function(move |_, arguments: mlua::MultiValue| {
                let failed = matches!(
                    arguments.front(),
                    None | Some(mlua::Value::Nil | mlua::Value::Boolean(false))
                );
                if failed
                    && arguments
                        .get(1)
                        .is_some_and(|v| !matches!(v, mlua::Value::String(_) | mlua::Value::Nil))
                {
                    return Err(mlua::Error::external(Error::Codec(
                        "Script execution failed.",
                    )));
                }
                original_assert.call::<mlua::MultiValue>(arguments)
            })?;
            globals.raw_set("assert", assertion)?;
            Ok(load_module(&lua, source).and_then(|module| call(&lua, module)))
        })
        .map_err(Error::from)
        .and_then(|result| result)
        .map_err(|cause| Error::Execution {
            cause: Box::new(cause),
            instructions: instructions.get(),
            peak_heap_bytes: memory.peak_heap_bytes(),
            allocation_bytes: memory.allocated_bytes(),
        })?;
    Ok((output, instructions.get(), memory))
}

fn lua_failure(stage: LuaFailureStage, cause: mlua::Error) -> Error {
    Error::LuaFailure {
        stage,
        cause: Box::new(Error::from(cause)),
    }
}

fn load_module(lua: &Lua, source: &[u8]) -> Result<Table, Error> {
    let compiled = lua
        .load(source)
        .set_name("mission")
        .set_mode(ChunkMode::Text)
        .into_function()
        .map_err(|cause| lua_failure(LuaFailureStage::Compilation, cause))?;
    let module = compiled
        .call::<mlua::Value>(())
        .map_err(|cause| lua_failure(LuaFailureStage::Execution, cause))?;
    match module {
        mlua::Value::Table(module) => Ok(module),
        _ => Err(Error::from(mlua::Error::RuntimeError(
            "Module must return a handler table.".into(),
        ))),
    }
}

fn to_lua(lua: &Lua, value: &Value) -> Result<mlua::Value, Error> {
    Ok(match value {
        Value::Null => mlua::Value::Nil,
        Value::Bool(value) => mlua::Value::Boolean(*value),
        Value::Integer(value) => mlua::Value::Integer(*value),
        Value::Number(value) => mlua::Value::Number(*value),
        Value::String(value) => mlua::Value::String(lua.create_string(value.as_bytes())?),
        Value::Array(values) => {
            let table = lua.create_table()?;
            for (index, value) in values.iter().enumerate() {
                table.raw_set(index + 1, to_lua(lua, value)?)?;
            }
            mlua::Value::Table(table)
        }
        Value::Map(values) => {
            let table = lua.create_table()?;
            for (key, value) in values {
                table.raw_set(key.as_str(), to_lua(lua, value)?)?;
            }
            mlua::Value::Table(table)
        }
    })
}
struct OutputBudget {
    items: usize,
    bytes: usize,
    limit: usize,
    ancestors: Vec<usize>,
}
impl OutputBudget {
    fn charge(&mut self, bytes: usize) -> Result<(), Error> {
        if bytes > self.limit.saturating_sub(self.bytes) {
            return Err(Error::Limit("Output budget exceeded."));
        }
        self.bytes += bytes;
        Ok(())
    }
}
fn from_lua(value: mlua::Value, depth: usize, budget: &mut OutputBudget) -> Result<Value, Error> {
    budget.items += 1;
    if depth >= MAX_DEPTH || budget.items > MAX_ITEMS {
        return Err(Error::Limit("Output nesting/items exceeded."));
    }
    budget.charge(1)?;
    match value {
        mlua::Value::Nil => Ok(Value::Null),
        mlua::Value::Boolean(value) => Ok(Value::Bool(value)),
        mlua::Value::Integer(value) => {
            budget.charge(8)?;
            Ok(Value::Integer(value))
        }
        mlua::Value::Number(value) => {
            if !value.is_finite() {
                return Err(Error::Codec("Nonfinite result."));
            }
            budget.charge(8)?;
            Ok(Value::Number(value))
        }
        mlua::Value::String(value) => {
            budget.charge(4 + value.as_bytes().len())?;
            Ok(Value::String(
                value
                    .to_str()
                    .map_err(|_| Error::Codec("Script strings must be UTF-8."))?
                    .to_owned(),
            ))
        }
        mlua::Value::Table(table) => from_table(table, depth, budget),
        _ => Err(Error::Codec("Result contains a nonserializable value.")),
    }
}
fn from_table(table: Table, depth: usize, budget: &mut OutputBudget) -> Result<Value, Error> {
    let identity = table.to_pointer() as usize;
    if budget.ancestors.contains(&identity) {
        return Err(Error::Codec("Cyclic result."));
    }
    budget.ancestors.push(identity);
    budget.charge(4)?;
    let mut count = 0_usize;
    let mut maximum = 0_usize;
    let mut strings = false;
    for pair in table.clone().pairs::<mlua::Value, mlua::Value>() {
        let (key, _) = pair?;
        count += 1;
        if count > MAX_ITEMS {
            return Err(Error::Limit("Result collection exceeded."));
        }
        match key {
            mlua::Value::String(_) => strings = true,
            mlua::Value::Integer(key) if key > 0 && key <= MAX_ITEMS as i64 => {
                maximum = maximum.max(key as usize);
            }
            _ => {
                return Err(Error::Codec(
                    "Result keys must be strings or dense positive indices.",
                ));
            }
        }
    }
    if (strings && maximum != 0) || (maximum != 0 && maximum != count) {
        return Err(Error::Codec("Mixed or sparse result table."));
    }
    let result = if maximum != 0 {
        let mut values = Vec::new();
        for index in 1..=count {
            values.push(from_lua(table.raw_get(index)?, depth + 1, budget)?);
        }
        Value::Array(values)
    } else {
        let mut values = BTreeMap::new();
        for pair in table.pairs::<mlua::String, mlua::Value>() {
            let (key, value) = pair?;
            budget.charge(4 + key.as_bytes().len())?;
            let key = key
                .to_str()
                .map_err(|_| Error::Codec("Script strings must be UTF-8."))?
                .to_owned();
            values.insert(key, from_lua(value, depth + 1, budget)?);
        }
        Value::Map(values)
    };
    budget.ancestors.pop();
    Ok(result)
}
