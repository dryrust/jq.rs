// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

#[test]
fn input_object_order_follows_consumer_features() -> Result<(), JsonFilterError> {
    let (keys, values) = if cfg!(feature = "preserve_order") {
        (json!(["z", "a"]), vec![json!(1), json!(2)])
    } else {
        (json!(["a", "z"]), vec![json!(2), json!(1)])
    };
    for (input, prefix) in [
        (r#"{"z":1,"a":2}"#, "."),
        (r#"{"nested":{"z":1,"a":2}}"#, ".nested"),
    ] {
        let value: Value = serde_json::from_str(input)?;
        let filter: JsonFilter = format!("{prefix} | keys_unsorted").parse()?;
        assert_eq!(filter.filter_json(value.clone())?, keys);
        assert_eq!(filter.filter_json_str(input)?, keys);
        let filter: JsonFilter = format!("{prefix} | .[]").parse()?;
        assert_eq!(filter.filter_json_all(value.clone())?, values);
        assert_eq!(filter.filter_json_str_all(input)?, values);
        let filter: JsonFilter = format!("{prefix} | keys").parse()?;
        assert_eq!(filter.filter_json(value)?, json!(["a", "z"]));
    }
    Ok(())
}

#[test]
fn output_objects_use_serde_json_map_order() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = r#"{nested: {z: 1, a: 2}}"#.parse()?;
    let output = filter.filter_json(json!(null))?;
    let keys: Vec<_> = output["nested"].as_object().unwrap().keys().collect();
    let expected = if cfg!(feature = "preserve_order") {
        ["z", "a"]
    } else {
        ["a", "z"]
    };
    assert_eq!(keys, expected);
    Ok(())
}
