// This is free and unencumbered software released into the public domain.

#![cfg(all(feature = "jaq", feature = "arbitrary_precision"))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

#[test]
fn consumer_precision_preserves_large_output_numbers() -> Result<(), JsonFilterError> {
    for literal in ["1e400", "18446744073709551617"] {
        let expected: Value = serde_json::from_str(literal)?;
        assert_eq!(expected.to_string(), literal);
        for program in [literal.to_owned(), format!("\"{literal}\" | fromjson")] {
            let filter: JsonFilter = program.parse()?;
            assert_eq!(filter.filter_json(Value::Null)?, expected);
            assert_eq!(filter.filter_json_str("null")?, expected);
            assert_eq!(
                filter.filter_json_all(Value::Null)?,
                core::slice::from_ref(&expected)
            );
            assert_eq!(
                filter.filter_json_str_all("null")?,
                core::slice::from_ref(&expected)
            );
        }
        let filter: JsonFilter = format!("{{nested: [{literal}]}}").parse()?;
        let nested = json!({"nested": [expected]});
        assert_eq!(filter.filter_json(Value::Null)?, nested);
        assert_eq!(filter.filter_json_all(Value::Null)?, [nested]);
    }
    Ok(())
}

#[test]
fn consumer_precision_preserves_input_values() -> Result<(), JsonFilterError> {
    let input = r#"{"large": [1e400, 18446744073709551617]}"#;
    let expected: Value = serde_json::from_str(input)?;
    let filter = JsonFilter::default();
    assert_eq!(filter.filter_json(expected.clone())?, expected);
    assert_eq!(filter.filter_json_str(input)?, expected);
    assert_eq!(filter.filter_json_str_all(input)?, [expected]);
    Ok(())
}
