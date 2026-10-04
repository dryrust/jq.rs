// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use core::ops::ControlFlow;
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn visitor_observes_values_before_an_error() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = r#"1, 2, error("later"), 3"#.parse()?;
    let mut seen = Vec::new();
    let result = filter.filter_json_visit(json!(null), |value| {
        seen.push(value);
        ControlFlow::<()>::Continue(())
    });
    assert_eq!(seen, [json!(1), json!(2)]);
    assert!(matches!(result, Err(JsonFilterError::Execute(_))));
    assert_eq!(
        filter.filter_json_visit(json!(null), ControlFlow::Break)?,
        ControlFlow::Break(json!(1))
    );
    Ok(())
}

#[test]
fn visitor_can_bound_infinite_output() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "repeat(.)".parse()?;
    let mut count = 0;
    let result = filter.filter_json_visit(json!("repeated"), |value| {
        assert_eq!(value, json!("repeated"));
        count += 1;
        if count == 5 {
            ControlFlow::Break(count)
        } else {
            ControlFlow::Continue(())
        }
    })?;
    assert_eq!(result, ControlFlow::Break(5));
    Ok(())
}

#[test]
fn visitor_distinguishes_exhaustion_from_early_stop() -> Result<(), JsonFilterError> {
    for (program, expected) in [("empty", vec![]), ("1, 2", vec![json!(1), json!(2)])] {
        let filter: JsonFilter = program.parse()?;
        let mut seen = Vec::new();
        let result = filter.filter_json_visit(json!(null), |value| {
            seen.push(value);
            ControlFlow::<()>::Continue(())
        })?;
        assert_eq!(result, ControlFlow::Continue(()));
        assert_eq!(seen, expected);
    }
    Ok(())
}
