// This is free and unencumbered software released into the public domain.

#![cfg(all(feature = "jq", not(feature = "jaq")))]

use core::ops::ControlFlow;
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn compiles_without_evaluating_the_program() -> Result<(), JsonFilterError> {
    for source in ["repeat(.)", "halt_error(5)", "def f: 1; f # comment"] {
        let _: JsonFilter = source.parse()?;
    }
    assert!(matches!(
        "[".parse::<JsonFilter>(),
        Err(JsonFilterError::Compile(_))
    ));
    assert!(matches!(
        "), error(\"validation executed\"), (".parse::<JsonFilter>(),
        Err(JsonFilterError::Compile(_))
    ));
    Ok(())
}

#[test]
fn missing_executable_reports_a_loading_diagnostic() {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "missing_executable_child",
            "--ignored",
            "--nocapture",
        ])
        .env("PATH", "")
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("missing jq handled"));
}

#[test]
#[ignore = "invoked in a subprocess with an empty PATH"]
fn missing_executable_child() {
    let Err(JsonFilterError::Compile(diagnostics)) = ".".parse::<JsonFilter>() else {
        panic!("expected a compilation loading failure");
    };
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].phase, jq::CompilationPhase::Load);
    assert!(diagnostics[0].message.contains("cannot run jq"));
    assert!(diagnostics[0].span.is_none());
    assert!(matches!(
        JsonFilter::default().filter_json(json!(null)),
        Err(JsonFilterError::Execute(_))
    ));
    println!("missing jq handled");
}

#[test]
fn streams_results_and_stops_an_infinite_process() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "repeat(.)".parse()?;
    assert_eq!(
        filter.filter_json_visit(json!(42), ControlFlow::Break)?,
        ControlFlow::Break(json!(42))
    );
    let filter: JsonFilter = r#"1, error("later")"#.parse()?;
    assert_eq!(filter.filter_json(json!(null))?, json!(1));
    assert!(matches!(
        filter.filter_json_all(json!(null)),
        Err(JsonFilterError::Execute(_))
    ));
    Ok(())
}

#[test]
fn drains_large_input_and_error_pipes() -> Result<(), JsonFilterError> {
    let input = json!("x".repeat(256 * 1024));
    assert_eq!(JsonFilter::default().filter_json(input.clone())?, input);
    let filter: JsonFilter = "error(.)".parse()?;
    let error = filter.filter_json(input).unwrap_err();
    assert!(matches!(error, JsonFilterError::Execute(_)));
    assert!(error.to_string().contains(&"x".repeat(1024)));
    Ok(())
}
