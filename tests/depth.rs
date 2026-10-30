// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

// Both backends accept exactly the nesting that serde_json can parse.
#[test]
fn results_nested_beyond_serde_json_limits_are_rejected() -> Result<(), JsonFilterError> {
    for (wrap, accepted) in [("[.]", 127), ("{a: .}", 127), ("[{a: .}]", 63)] {
        for depth in [accepted, accepted + 1] {
            let source = format!("reduce range({depth}) as $i (null; {wrap})");
            let filter: JsonFilter = source.parse()?;
            let result = filter.filter_json(Value::Null);
            if depth == accepted {
                let text = serde_json::to_string(&result?).unwrap();
                assert!(serde_json::from_str::<Value>(&text).is_ok(), "{source}");
            } else {
                let Err(JsonFilterError::Output(error)) = result else {
                    panic!("{source}: {result:?}");
                };
                assert!(error.to_string().contains("recursion limit"), "{source}");
            }
        }
    }
    // The limit applies to each result, not to the program's intermediates.
    let filter: JsonFilter = "reduce range(1000) as $i (null; [.]) | [paths] | length".parse()?;
    assert_eq!(filter.filter_json(Value::Null)?, json!(1000));
    Ok(())
}
