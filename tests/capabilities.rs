// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn core_pipeline_and_rounding_work_without_std() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "map(select(. >= 0)) | sort | map(. + 1)".parse()?;
    assert_eq!(filter.filter_json(json!([3, -1, 0, 2]))?, json!([1, 3, 4]));
    let filter: JsonFilter = "[floor, ceil, round]".parse()?;
    for (input, expected) in [
        (json!(-1.5), json!([-2, -1, -2])),
        (json!(1.5), json!([1, 2, 2])),
        (json!(-f64::from_bits(1)), json!([-1, 0, 0])),
    ] {
        assert_eq!(filter.filter_json(input)?, expected);
    }
    let filter: JsonFilter = "explode | implode".parse()?;
    assert_eq!(filter.filter_json(json!("é日本語🦀"))?, json!("é日本語🦀"));
    let filter: JsonFilter = "split(\",\")".parse()?;
    assert_eq!(filter.filter_json(json!("a,b"))?, json!(["a", "b"]));
    Ok(())
}

#[test]
fn optional_native_functions_follow_the_std_feature() {
    for source in [
        "now",
        "env",
        "sin",
        "test(\"x\")",
        "@base64",
        "fromdateiso8601",
        "debug",
    ] {
        let compiled = source.parse::<JsonFilter>();
        if cfg!(feature = "std") {
            assert!(compiled.is_ok(), "{source}");
        } else {
            assert!(
                matches!(compiled, Err(JsonFilterError::Compile(_))),
                "{source}"
            );
        }
    }
}
