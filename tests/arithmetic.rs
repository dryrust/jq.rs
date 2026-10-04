// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn addition_promotes_past_the_machine_integer_limit() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = ". + 1".parse()?;
    let input = json!(isize::MAX);
    let expected = json!((isize::MAX as u64) + 1);
    assert_eq!(filter.filter_json(input.clone())?, expected);
    assert_eq!(filter.filter_json_all(input)?, [expected]);
    Ok(())
}
