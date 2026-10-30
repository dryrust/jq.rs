// This is free and unencumbered software released into the public domain.

#![cfg(all(feature = "jaq", feature = "std"))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Map, Value, json};

#[test]
fn env_exposes_host_variables_only_on_opt_in() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "env".parse()?;
    let expected = if cfg!(feature = "env") {
        std::env::vars().map(|(k, v)| (k, json!(v))).collect()
    } else {
        Map::new()
    };
    assert_eq!(filter.filter_json(Value::Null)?, Value::Object(expected));
    Ok(())
}
