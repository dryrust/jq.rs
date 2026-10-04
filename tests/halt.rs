// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::Value;

#[test]
fn halt_returns_to_the_embedding_process() {
    for program in ["halt", "halt_error", "halt_error(5)"] {
        for method in ["value", "string", "values", "strings"] {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "halt_child", "--ignored", "--nocapture"])
                .env("JQ_TEST_HALT_PROGRAM", program)
                .env("JQ_TEST_HALT_METHOD", method)
                .output()
                .unwrap();
            assert!(output.status.success(), "{program}: {output:?}");
            // Exit status zero alone would also accept the original halt bug.
            assert!(
                String::from_utf8_lossy(&output.stdout).contains("returned safely"),
                "{program} ({method}): {output:?}"
            );
        }
    }
}

#[test]
#[ignore = "invoked in a subprocess by halt_returns_to_the_embedding_process"]
fn halt_child() {
    let program = std::env::var("JQ_TEST_HALT_PROGRAM").unwrap();
    let filter: JsonFilter = program.parse().unwrap();
    let error = match std::env::var("JQ_TEST_HALT_METHOD").unwrap().as_str() {
        "value" => filter.filter_json(Value::Null).unwrap_err(),
        "string" => filter.filter_json_str("null").unwrap_err(),
        "values" => filter.filter_json_all(Value::Null).unwrap_err(),
        "strings" => filter.filter_json_str_all("null").unwrap_err(),
        _ => unreachable!(),
    };
    assert!(matches!(error, JsonFilterError::Execute(_)));
    assert!(
        error
            .to_string()
            .contains("process termination is disabled")
    );
    println!("returned safely");
}
