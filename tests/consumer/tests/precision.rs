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

#[test]
fn computed_big_integers_remain_exact() -> Result<(), JsonFilterError> {
    for (source, decimal) in [
        ("18446744073709551615 + 2", "18446744073709551617"),
        ("-9223372036854775808 - 1", "-9223372036854775809"),
        (
            "18446744073709551615 * 18446744073709551615",
            "340282366920938463426481119284349108225",
        ),
    ] {
        let expected: Value = serde_json::from_str(decimal)?;
        let filter: JsonFilter = source.parse()?;
        assert_eq!(filter.filter_json(Value::Null)?.to_string(), decimal);
        assert_eq!(
            filter.filter_json_all(Value::Null)?,
            core::slice::from_ref(&expected)
        );
        let nested: JsonFilter = format!("{{computed: [({source})]}}").parse()?;
        assert_eq!(
            nested.filter_json_str("null")?,
            json!({"computed": [expected]})
        );
    }
    Ok(())
}

#[test]
fn big_integer_inputs_and_bindings_retain_unit_differences() -> Result<(), JsonFilterError> {
    let large: Value = serde_json::from_str("18446744073709551617")?;
    let expected: Value = serde_json::from_str("18446744073709551618")?;
    let filter: JsonFilter = ". + 1".parse()?;
    assert_eq!(filter.filter_json(large.clone())?, expected);
    let bound = JsonFilter::with_bindings("$large + 1", [("large", large.clone())])?;
    assert_eq!(bound.filter_json(Value::Null)?, expected);
    let streamed: JsonFilter = "input + 1".parse()?;
    assert_eq!(
        streamed.filter_json_with_inputs(Value::Null, [Ok(large)])?,
        expected
    );
    Ok(())
}
