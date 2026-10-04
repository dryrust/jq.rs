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
