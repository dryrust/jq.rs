// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

fn assert_number(program: &str, expected: &str) -> Result<(), JsonFilterError> {
    let filter: JsonFilter = program.parse()?;
    let expected: Value = serde_json::from_str(expected)?;
    assert_eq!(filter.filter_json(json!(null))?, expected, "{program}");
    assert_eq!(filter.filter_json_str("null")?, expected, "{program}");
    assert_eq!(
        filter.filter_json_all(json!(null))?,
        core::slice::from_ref(&expected),
        "{program}"
    );
    assert_eq!(filter.filter_json_str_all("null")?, [expected], "{program}");
    Ok(())
}

#[test]
fn addition_promotes_past_the_machine_integer_limit() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = ". + 1".parse()?;
    let input = json!(isize::MAX);
    let expected = json!((isize::MAX as u64) + 1);
    assert_eq!(filter.filter_json(input.clone())?, expected);
    assert_eq!(filter.filter_json_all(input)?, [expected]);
    Ok(())
}

#[test]
fn addition_and_subtraction_preserve_exact_intermediates() -> Result<(), JsonFilterError> {
    let max = isize::MAX as i128;
    let min = isize::MIN as i128;
    for (program, expected) in [
        (format!("{max} + 1"), (max + 1).to_string()),
        (format!("{min} - 1"), (min - 1).to_string()),
        (format!("({max} + 1) - 1"), max.to_string()),
        (format!("({min} - 1) + 1"), min.to_string()),
        (format!("{min} + (-1)"), (min - 1).to_string()),
        (format!("{max} - (-1)"), (max + 1).to_string()),
    ] {
        assert_number(&program, &expected)?;
    }
    // These intermediates exceed the JSON integer range even on 64-bit hosts.
    assert_number("(18446744073709551615 + 1) - 1", "18446744073709551615")?;
    assert_number("(-9223372036854775808 - 1) + 1", "-9223372036854775808")?;
    assert_number("18446744073709551616 - 18446744073709551615", "1")?;
    Ok(())
}

#[test]
fn multiplication_preserves_large_intermediates() -> Result<(), JsonFilterError> {
    let max = isize::MAX as i128;
    let min = isize::MIN as i128;
    for (left, right) in [(max, 2), (min, -1), (max, max), (min, min)] {
        assert_number(&format!("{left} * {right}"), &(left * right).to_string())?;
    }
    assert_number(
        &format!("({max} * {max}) - ({max} * ({max} - 1))"),
        &max.to_string(),
    )?;
    Ok(())
}

#[test]
fn multiplication_uses_exact_input_and_binding_values() -> Result<(), JsonFilterError> {
    let max = isize::MAX;
    let expected = json!((max as u64) * 2);
    let filter: JsonFilter = ".[0] * .[1]".parse()?;
    let input = json!([max, 2]);
    assert_eq!(filter.filter_json(input.clone())?, expected);
    assert_eq!(filter.filter_json_str(input.to_string())?, expected);
    assert_eq!(
        filter.filter_json_all(input.clone())?,
        core::slice::from_ref(&expected)
    );
    assert_eq!(
        filter.filter_json_str_all(input.to_string())?,
        core::slice::from_ref(&expected)
    );
    let bound = JsonFilter::with_bindings(". * $factor", [("factor", json!(2))])?;
    assert_eq!(bound.filter_json(json!(max))?, expected);
    let streamed: JsonFilter = ". * input".parse()?;
    assert_eq!(
        streamed.filter_json_with_inputs(json!(max), [Ok(json!(2))])?,
        expected
    );
    Ok(())
}

#[test]
fn minimum_integer_absolute_value_and_length_are_positive() -> Result<(), JsonFilterError> {
    let min = isize::MIN;
    let expected = json!(min.unsigned_abs());
    for source in ["abs", "length"] {
        let filter: JsonFilter = source.parse()?;
        assert_eq!(filter.filter_json(json!(min))?, expected);
        assert_eq!(
            filter.filter_json_all(json!(min))?,
            core::slice::from_ref(&expected)
        );
        assert_number(
            &format!("{min} | {source}"),
            &min.unsigned_abs().to_string(),
        )?;
    }
    Ok(())
}

#[test]
fn length_retains_non_integer_behavior() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "length".parse()?;
    for (input, expected) in [
        (json!(null), json!(0)),
        (json!("é🦀"), json!(2)),
        (json!([1, 2, 3]), json!(3)),
        (json!({"a": 1}), json!(1)),
        (json!(-2.5), json!(2.5)),
    ] {
        assert_eq!(filter.filter_json(input)?, expected);
    }
    assert!(matches!(
        filter.filter_json(json!(true)),
        Err(JsonFilterError::Execute(_))
    ));
    assert_number("-18446744073709551615 | length", "18446744073709551615")?;
    Ok(())
}
