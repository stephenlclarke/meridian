//! Executes the Root-audited, G2-authored frozen corpus without changing its expectations.
mod support;
use meridian_missions::{
    Error, Limits, LuaFailureStage, Value, decode, decode_tagged, encode, evaluate, validate,
};
use serde_json::{Value as Json, json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use support::Required;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/compat")
}
fn read_json(path: &Path) -> Json {
    serde_json::from_slice(&std::fs::read(path).required()).required()
}
fn value(raw: &Json) -> Value {
    match raw["type"].as_str().required() {
        "null" => Value::Null,
        "bool" => Value::Bool(raw["value"].as_bool().required()),
        "integer" => Value::Integer(raw["value"].as_i64().required()),
        // Fixture inputs intentionally include nonfinite bits for negative encoder tests.
        "number" => Value::Number(f64::from_bits(
            u64::from_str_radix(raw["value"].as_str().required(), 16).required(),
        )),
        "string" => Value::String(raw["value"].as_str().required().to_owned()),
        "array" => Value::Array(
            raw["value"]
                .as_array()
                .required()
                .iter()
                .map(value)
                .collect(),
        ),
        "object" => Value::Map(
            raw["value"]
                .as_object()
                .required()
                .iter()
                .map(|(k, v)| (k.clone(), value(v)))
                .collect::<BTreeMap<_, _>>(),
        ),
        other => panic!("unknown frozen fixture value {other}"),
    }
}
fn canonical(value: &Value) -> Json {
    match value {
        Value::Null => json!({"type":"null"}),
        Value::Bool(v) => json!({"type":"bool","value":v}),
        Value::Integer(v) => json!({"type":"integer","value":v}),
        Value::Number(v) => json!({"type":"number","value":format!("{:016x}",v.to_bits())}),
        Value::String(v) => json!({"type":"string","value":v}),
        Value::Array(v) => {
            json!({"type":"array","value":v.iter().map(canonical).collect::<Vec<_>>()})
        }
        Value::Map(v) => {
            json!({"type":"object","value":v.iter().map(|(k,v)| (k.clone(),canonical(v))).collect::<serde_json::Map<_,_>>()})
        }
    }
}
fn limits(invocation: &Json) -> Limits {
    let mut result = Limits::default();
    if let Some(values) = invocation["limitsOverride"].as_object() {
        for (key, value) in values {
            let number = value.as_u64().unwrap_or(u64::MAX);
            match key.as_str() {
                "instructions" => result.instructions = number,
                "heapBytes" => result.heap_bytes = number as usize,
                "allocationBytes" => result.allocation_bytes = number as usize,
                "outputBytes" => result.output_bytes = number as usize,
                key => panic!("unhandled frozen execution limit {key}"),
            }
        }
    }
    result
}
fn expect_result(result: &Result<Json, Error>, expected: &Json, case: &str) {
    if expected["status"] == "error" {
        let error = result
            .as_ref()
            .err()
            .unwrap_or_else(|| panic!("{case}: expected rejection, got {result:?}"));
        let text = error.to_string();
        match expected["error"]["match"].as_str().required() {
            "exact" => assert_eq!(
                text,
                expected["error"]["text"].as_str().required(),
                "{case}"
            ),
            "contains" => assert!(
                text.contains(expected["error"]["text"].as_str().required()),
                "{case}: {text}"
            ),
            "category" => assert_eq!(
                error.lua_failure_stage(),
                Some(expected_failure_stage(expected)),
                "{case}: wrong Lua failure stage: {error:?}"
            ),
            other => panic!("unhandled frozen error matcher {other}"),
        }
        if let Some(exact) = expected["metrics"]["instructions"]["exact"].as_u64() {
            assert_eq!(error.instruction_count(), Some(exact), "{case}: hook count");
        }
    } else {
        let actual = result.as_ref().unwrap_or_else(|e| panic!("{case}: {e}"));
        for field in ["data", "commands", "value"] {
            if !expected[field].is_null() {
                assert_eq!(actual[field], expected[field], "{case}: {field}");
            }
        }
    }
}
fn expected_failure_stage(expected: &Json) -> LuaFailureStage {
    match expected["error"]["text"].as_str().required() {
        "Script compilation error" => LuaFailureStage::Compilation,
        "Script execution error" => LuaFailureStage::Execution,
        other => panic!("unhandled frozen error category {other}"),
    }
}

fn run(id: &str) {
    let fixture = read_json(&root().join("cases").join(format!("{id}.json")));
    let invocation = &fixture["invocation"];
    let operation = invocation["operation"].as_str().required();
    let started = std::time::Instant::now();
    let mut successful_data = None;
    let mut successful_metrics = None;
    let result: Result<Json, Error> = match operation {
        "evaluate" => {
            let source =
                std::fs::read(root().join(invocation["source"].as_str().required())).required();
            let event = value(&invocation["event"]);
            let snapshot = value(&invocation["snapshot"]);
            let state = value(&invocation["state"]);
            let before = canonical(&state);
            let result = evaluate(
                &source,
                invocation["handler"].as_str().required(),
                &event,
                &snapshot,
                &state,
                limits(invocation),
            );
            assert_eq!(canonical(&state), before, "{id}: input state changed");
            result.map(|result| {
                successful_data = Some(result.data.clone());
                successful_metrics = Some(json!({"instructions":result.instructions,"peakHeapBytes":result.peak_heap_bytes,"allocationBytes":result.allocation_bytes}));
                json!({"data":canonical(&result.data),"commands":result.commands.iter().map(canonical).collect::<Vec<_>>()})
            })
        }
        "validate" => {
            let source =
                std::fs::read(root().join(invocation["source"].as_str().required())).required();
            let handlers = invocation["declaredHandlers"]
                .as_array()
                .required()
                .iter()
                .map(|v| v.as_str().required().to_owned())
                .collect::<Vec<_>>();
            validate(&source, &handlers).map(|_| json!({}))
        }
        "wire_decode" => {
            decode(&std::fs::read(root().join(invocation["bytes"].as_str().required())).required())
                .map(|v| json!({"value":canonical(&v)}))
        }
        "wire_encode" => encode(
            &[value(&invocation["value"])],
            invocation["byteLimit"].as_u64().unwrap_or(1_048_576) as usize,
        )
        .map(|_| json!({})),
        "tagged_storage_decode" => {
            decode_tagged(&invocation["value"]).map(|v| json!({"value":canonical(&v)}))
        }
        other => panic!("unhandled frozen corpus operation {other}"),
    };
    let wall_seconds = started.elapsed().as_secs_f64();
    println!(
        "{}",
        json!({"luaCompatibilityCase":id,"source":option_env!("MERIDIAN_SOURCE_SHA").unwrap_or("UNBOUND"),
        "corpusManifestSHA256":"3da46b069b5175fe5c5e4a9831fdac391c4de6b4bffe4259132c99800c9929d9",
        "runtime":"mlua0.12.2 with retained budget/seed patch; lua-src551.1.0/Lua5.5.1",
        "result":result.as_ref().ok(),"error":result.as_ref().err().map(ToString::to_string),
        "wallSeconds":wall_seconds,"referenceWallSeconds":"UNRUN",
        "metrics":successful_metrics,"failureInstructions":result.as_ref().err().and_then(Error::instruction_count),
        "failureStage":result.as_ref().err().and_then(Error::lua_failure_stage).map(|stage|format!("{stage:?}"))})
    );
    expect_result(&result, &fixture["expected"], id);
    if let Some(followup) = fixture.get("followup") {
        let followup_started = std::time::Instant::now();
        let path = followup["source"]
            .as_str()
            .or_else(|| invocation["source"].as_str())
            .required();
        let source = std::fs::read(root().join(path)).required();
        let original = value(&invocation["state"]);
        let state = if followup["stateFromPreviousData"] == true {
            successful_data.as_ref().required()
        } else {
            &original
        };
        let result = evaluate(&source, invocation["handler"].as_str().required(), &value(&invocation["event"]),
            &value(&invocation["snapshot"]), state, Limits::default()).map(|r| json!({"data":canonical(&r.data),"commands":r.commands.iter().map(canonical).collect::<Vec<_>>()}));
        let mut expected = followup["expected"].clone();
        expected
            .as_object_mut()
            .required()
            .insert("status".into(), json!("ok"));
        expect_result(&result, &expected, &format!("{id} followup"));
        println!(
            "{}",
            json!({"luaCompatibilityFollowup":id,"wallSeconds":followup_started.elapsed().as_secs_f64(),
            "source":option_env!("MERIDIAN_SOURCE_SHA").unwrap_or("UNBOUND"),"result":result.as_ref().ok()})
        );
    }
}

#[test]
fn frozen_corpus_manifest_and_every_payload_are_unchanged() {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(root().join("MANIFEST.json")).required();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        "3da46b069b5175fe5c5e4a9831fdac391c4de6b4bffe4259132c99800c9929d9"
    );
    let manifest: Json = serde_json::from_slice(&bytes).required();
    let payloads = manifest["payloads"].as_array().required();
    assert_eq!(payloads.len(), 382);
    for payload in payloads {
        let bytes = std::fs::read(root().join(payload["path"].as_str().required())).required();
        assert_eq!(bytes.len() as u64, payload["bytes"].as_u64().required());
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            payload["sha256"].as_str().required()
        );
    }
}

#[test]
fn frozen_category_matcher_rejects_unrelated_and_opposite_stage_errors() {
    for id in [
        "binary-chunk-refused",
        "syntax-error",
        "allocation-bound-refusal",
        "recursive-runtime-failure",
    ] {
        let fixture = read_json(&root().join("cases").join(format!("{id}.json")));
        let expected = &fixture["expected"];
        assert_eq!(expected["error"]["match"], "category");
        let stage = expected_failure_stage(expected);
        let opposite = match stage {
            LuaFailureStage::Compilation => LuaFailureStage::Execution,
            LuaFailureStage::Execution => LuaFailureStage::Compilation,
        };
        let errors = [
            Error::Replay,
            Error::Limit("Invalid script input or limits."),
            Error::Pack("Missing named handler."),
            Error::Codec("Result contains a nonserializable value."),
            // Diagnostic text and the pre-existing metrics wrapper cannot manufacture provenance.
            Error::Lua("Script compilation error; Script execution error".into()),
            Error::Execution {
                cause: Box::new(Error::Replay),
                instructions: 0,
                peak_heap_bytes: 0,
                allocation_bytes: 0,
            },
            Error::LuaFailure {
                stage: opposite,
                cause: Box::new(Error::Lua("platform-specific diagnostic".into())),
            },
        ];
        for error in errors {
            let result: Result<Json, Error> = Err(error);
            assert!(
                std::panic::catch_unwind(|| expect_result(&result, expected, id)).is_err(),
                "{id}: wrong-stage error was incorrectly credited: {result:?}"
            );
        }
        // Matching typed provenance must also pass; an always-rejecting matcher is insufficient.
        let matching = Err(Error::Execution {
            cause: Box::new(Error::LuaFailure {
                stage,
                cause: Box::new(Error::Lua("platform-specific diagnostic".into())),
            }),
            instructions: 0,
            peak_heap_bytes: 0,
            allocation_bytes: 0,
        });
        expect_result(&matching, expected, id);
    }
}

#[test]
fn real_text_compiler_errors_keep_provenance_through_validator_and_evaluator() {
    for id in ["binary-chunk-refused", "syntax-error"] {
        let fixture = read_json(&root().join("cases").join(format!("{id}.json")));
        let source =
            std::fs::read(root().join(fixture["invocation"]["source"].as_str().required()))
                .required();
        let error = validate(&source, &["on_event".into()]).err().required();
        assert_eq!(
            error.lua_failure_stage(),
            Some(LuaFailureStage::Compilation)
        );
        let error = evaluate(
            &source,
            "on_event",
            &Value::empty_map(),
            &Value::empty_map(),
            &Value::empty_map(),
            Limits::default(),
        )
        .err()
        .required();
        assert_eq!(
            error.lua_failure_stage(),
            Some(LuaFailureStage::Compilation)
        );
    }
}

#[test]
fn real_module_and_handler_calls_are_distinct_from_preflight_and_host_validation() {
    let module_error = b"error('module call diagnostic')";
    let handler_error = b"return {run=function(e,w,d) error('handler call diagnostic') end}";
    for source in [module_error.as_slice(), handler_error.as_slice()] {
        let error = evaluate(
            source,
            "run",
            &Value::empty_map(),
            &Value::empty_map(),
            &Value::empty_map(),
            Limits::default(),
        )
        .err()
        .required();
        assert_eq!(error.lua_failure_stage(), Some(LuaFailureStage::Execution));
    }
    let error = validate(module_error, &["run".into()]).err().required();
    assert_eq!(error.lua_failure_stage(), Some(LuaFailureStage::Execution));
    let missing = validate(b"return {}", &["run".into()]).err().required();
    assert_eq!(missing.lua_failure_stage(), None);
    let valid = b"return {run=function(e,w,d) return {data=d,commands={}} end}";
    for (source, handler, limits) in [
        (valid.as_slice(), "invalid.handler", Limits::default()),
        (
            valid.as_slice(),
            "run",
            Limits {
                instructions: 1,
                ..Limits::default()
            },
        ),
        (b"return 2".as_slice(), "run", Limits::default()),
        (
            b"return {run=function() return {data=false,commands={}} end}".as_slice(),
            "run",
            Limits::default(),
        ),
        (
            b"return {run=function() return {data={bad=function()end},commands={}} end}".as_slice(),
            "run",
            Limits::default(),
        ),
    ] {
        let error = evaluate(
            source,
            handler,
            &Value::empty_map(),
            &Value::empty_map(),
            &Value::empty_map(),
            limits,
        )
        .err()
        .required();
        assert_eq!(error.lua_failure_stage(), None, "{error:?}");
    }
}

// One named test per frozen invocation preserves failures and permits owned focused re-execution.
include!("support/compat_cases.rs");
