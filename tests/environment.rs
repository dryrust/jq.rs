// This is free and unencumbered software released into the public domain.

#![cfg(all(any(feature = "jaq", feature = "jq"), feature = "std"))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Map, Value, json};

#[test]
fn env_exposes_host_variables_only_on_opt_in() -> Result<(), JsonFilterError> {
    let expected = Value::Object(if cfg!(feature = "env") {
        std::env::vars().map(|(k, v)| (k, json!(v))).collect()
    } else {
        Map::new()
    });
    // jaq does not define `$ENV`.
    let sources: &[&str] = if cfg!(feature = "jaq") {
        &["env"]
    } else {
        &["env", "$ENV"]
    };
    for source in sources {
        let filter: JsonFilter = source.parse()?;
        assert_eq!(filter.filter_json(Value::Null)?, expected, "{source}");
    }
    Ok(())
}
