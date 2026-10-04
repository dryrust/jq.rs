// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn compiled_and_cloned_filters_can_be_reused() -> Result<(), JsonFilterError> {
    let original: JsonFilter = ".users[] | select(.active) | .profile.name".parse()?;
    let cloned = original.clone();
    for name in ["Zoë", "日本語 🦀", "مرحبا"] {
        let input = json!({"users": [
            {"active": false, "profile": {"name": "excluded"}},
            {"active": true, "profile": {"name": name}}
        ]});
        for filter in [&original, &cloned] {
            assert_eq!(filter.filter_json(input.clone())?, json!(name));
            assert_eq!(filter.filter_json_str(input.to_string())?, json!(name));
            assert_eq!(filter.filter_json_all(input.clone())?, [json!(name)]);
            assert_eq!(filter.filter_json_str_all(input.to_string())?, [json!(name)]);
        }
    }
    Ok(())
}

#[test]
fn default_filter_is_identity() -> Result<(), JsonFilterError> {
    let filter = JsonFilter::default();
    for input in [
        json!(null),
        json!({"nested": [true, "日本語 🦀"]}),
        json!(i64::MIN),
        json!(i64::MAX),
        json!(u64::MAX),
    ] {
        assert_eq!(filter.filter_json(input.clone())?, input);
        assert_eq!(filter.filter_json_str(input.to_string())?, input);
        assert_eq!(filter.filter_json_all(input.clone())?, [input.clone()]);
        assert_eq!(filter.filter_json_str_all(input.to_string())?, [input]);
    }
    Ok(())
}

#[test]
fn invalid_programs_report_compilation_diagnostics() {
    for program in ["[", "1 +", "unknown_function", "$unbound"] {
        match program.parse::<JsonFilter>() {
            Err(JsonFilterError::Compile(diagnostics)) => {
                assert!(!diagnostics.is_empty(), "{program}");
                assert!(diagnostics.iter().all(|message| !message.is_empty()));
            }
            _ => panic!("expected compilation diagnostics for {program}"),
        }
    }
}

#[test]
fn collects_all_results_in_order() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = ".[]".parse()?;
    let input = json!([1, null, false, {"name": "example"}, [2, 3]]);
    let expected = input.as_array().unwrap();
    assert_eq!(&filter.filter_json_all(input.clone())?, expected);
    assert_eq!(&filter.filter_json_str_all(input.to_string())?, expected);
    Ok(())
}

#[test]
fn empty_output_is_an_empty_collection() -> Result<(), JsonFilterError> {
    for expression in ["empty", "select(false)", ".[]"] {
        let filter: JsonFilter = expression.parse()?;
        assert!(filter.filter_json_all(json!([]))?.is_empty());
        assert!(filter.filter_json_str_all("[]")?.is_empty());
    }
    Ok(())
}

#[test]
fn reports_execution_errors_before_and_after_results() -> Result<(), JsonFilterError> {
    for expression in [r#"error("failure")"#, r#".[], error("failure")"#] {
        let filter: JsonFilter = expression.parse()?;
        for result in [
            filter.filter_json_all(json!([1, 2])),
            filter.filter_json_str_all("[1, 2]"),
        ] {
            let error = result.unwrap_err();
            assert!(matches!(error, JsonFilterError::Execute(_)));
            assert!(error.to_string().contains("failure"));
        }
    }
    Ok(())
}

#[test]
fn rejects_invalid_json_and_multiple_inputs() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = ".".parse()?;
    for input in ["", "{invalid}", "1 2"] {
        assert!(matches!(
            filter.filter_json_str_all(input),
            Err(JsonFilterError::Parse(_))
        ));
    }
    Ok(())
}

#[test]
fn single_result_methods_keep_their_original_behavior() -> Result<(), JsonFilterError> {
    for expression in [".[]", r#".[], error("later")"#] {
        let filter: JsonFilter = expression.parse()?;
        assert_eq!(filter.filter_json(json!([1, 2]))?, json!(1));
        assert_eq!(filter.filter_json_str("[1, 2]")?, json!(1));
    }
    let filter: JsonFilter = "empty".parse()?;
    assert!(matches!(
        filter.filter_json(json!(null)),
        Err(JsonFilterError::NoOutput)
    ));
    assert!(matches!(
        filter.filter_json_str("null"),
        Err(JsonFilterError::NoOutput)
    ));
    Ok(())
}
