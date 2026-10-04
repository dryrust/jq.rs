// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use core::ops::ControlFlow;
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn auxiliary_inputs_work_with_all_result_apis() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "input, inputs".parse()?;
    let auxiliary = [json!(1), json!({"nested": [true, null, "日本語"]})];
    let values = || auxiliary.iter().cloned().map(Ok);
    assert_eq!(
        filter.filter_json_with_inputs(json!("main"), values())?,
        json!(1)
    );
    assert_eq!(
        filter.filter_json_all_with_inputs(json!("main"), values())?,
        auxiliary
    );
    let mut seen = Vec::new();
    let status = filter.filter_json_visit_with_inputs(json!("main"), values(), |value| {
        seen.push(value);
        ControlFlow::<()>::Continue(())
    })?;
    assert_eq!(status, ControlFlow::Continue(()));
    assert_eq!(seen, auxiliary);
    Ok(())
}

#[test]
fn auxiliary_inputs_coexist_with_rebound_variables() -> Result<(), JsonFilterError> {
    let mut filter = JsonFilter::with_bindings("[$offset, ., inputs]", [("offset", json!(1))])?;
    for offset in [json!(2), json!(3)] {
        filter.set_bindings([("offset", offset.clone())])?;
        assert_eq!(
            filter.filter_json_with_inputs(json!(0), [Ok(json!(4)), Ok(json!(5))])?,
            json!([offset, 0, 4, 5])
        );
    }
    Ok(())
}
