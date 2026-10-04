// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

#[test]
fn nonfinite_results_become_json_null() -> Result<(), JsonFilterError> {
    for program in ["nan", "infinite", "-infinite", "1 / 0", "0 / 0"] {
        for (program, expected) in [
            (program.to_owned(), Value::Null),
            (
                format!("[{{number: ({program})}}]"),
                json!([{"number": null}]),
            ),
        ] {
            let filter: JsonFilter = program.parse()?;
            assert_eq!(filter.filter_json(Value::Null)?, expected);
            assert_eq!(filter.filter_json_str("null")?, expected);
            assert_eq!(
                filter.filter_json_all(Value::Null)?,
                core::slice::from_ref(&expected)
            );
            assert_eq!(filter.filter_json_str_all("null")?, [expected]);
        }
    }
    Ok(())
}

#[test]
fn large_literals_follow_serde_json_number_precision() -> Result<(), JsonFilterError> {
    let literal = "18446744073709551617";
    let expected: Value = serde_json::from_str(literal)?;
    let filter: JsonFilter = literal.parse()?;
    assert_eq!(filter.filter_json(Value::Null)?, expected);
    assert_eq!(filter.filter_json_all(Value::Null)?, [expected]);
    Ok(())
}

#[test]
fn unrepresentable_output_returns_an_error() -> Result<(), JsonFilterError> {
    // Consumer feature unification may make this number representable.
    let number = "1e400".parse::<serde_json::Number>();
    for program in [
        "1e400",
        "[1e400]",
        "{nested: [1e400]}",
        r#""1e400" | fromjson"#,
    ] {
        let filter: JsonFilter = program.parse()?;
        let expected = number.as_ref().ok().map(|number| match program {
            "[1e400]" => json!([number]),
            "{nested: [1e400]}" => json!({"nested": [number]}),
            _ => Value::Number(number.clone()),
        });
        for result in [
            filter.filter_json(Value::Null),
            filter.filter_json_str("null"),
        ] {
            match &expected {
                Some(expected) => assert_eq!(result?, *expected),
                None => assert!(matches!(result, Err(JsonFilterError::Output(_)))),
            }
        }
        for result in [
            filter.filter_json_all(Value::Null),
            filter.filter_json_str_all("null"),
        ] {
            match &expected {
                Some(expected) => assert_eq!(result?, core::slice::from_ref(expected)),
                None => assert!(matches!(result, Err(JsonFilterError::Output(_)))),
            }
        }
    }
    Ok(())
}

#[test]
fn single_result_does_not_convert_later_outputs() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "1, 1e400".parse()?;
    assert_eq!(filter.filter_json(Value::Null)?, json!(1));
    if "1e400".parse::<serde_json::Number>().is_err() {
        assert!(matches!(
            filter.filter_json_all(Value::Null),
            Err(JsonFilterError::Output(_))
        ));
    }
    Ok(())
}
