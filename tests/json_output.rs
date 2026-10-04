// This is free and unencumbered software released into the public domain.

#![cfg(feature = "jaq")]

use core::ops::ControlFlow;
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn non_json_values_return_conversion_errors() -> Result<(), JsonFilterError> {
    for source in [
        "[65, 66] | tobytes",
        "[-255] | implode",
        "{(1): true}",
        "{(1e400): true}",
        "{([-255] | implode): true}",
        "{nested: [{([1]): true}]}",
        "{nested: [([65] | tobytes)]}",
        "[([-255] | implode)]",
    ] {
        let filter: JsonFilter = source.parse()?;
        for result in [
            filter.filter_json(json!(null)),
            filter.filter_json_str("null"),
        ] {
            assert!(
                matches!(result, Err(JsonFilterError::OutputValue(_))),
                "{source}: {result:?}"
            );
        }
        for result in [
            filter.filter_json_all(json!(null)),
            filter.filter_json_str_all("null"),
        ] {
            assert!(
                matches!(result, Err(JsonFilterError::OutputValue(_))),
                "{source}: {result:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn conversion_failures_follow_the_output_consumption_contract() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "1, ([65] | tobytes), 2".parse()?;
    assert_eq!(filter.filter_json(json!(null))?, json!(1));
    assert!(matches!(
        filter.filter_json_all(json!(null)),
        Err(JsonFilterError::OutputValue(_))
    ));
    let mut seen = Vec::new();
    let result = filter.filter_json_visit(json!(null), |value| {
        seen.push(value);
        ControlFlow::<()>::Continue(())
    });
    assert!(matches!(result, Err(JsonFilterError::OutputValue(_))));
    assert_eq!(seen, [json!(1)]);
    Ok(())
}

#[test]
fn invalid_codepoints_are_execution_errors() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "implode".parse()?;
    for value in [isize::MIN as i64, isize::MAX as i64, -256, 0xd800, 0x110000] {
        let result = filter.filter_json(json!([value]));
        assert!(
            matches!(result, Err(JsonFilterError::Execute(_))),
            "{value}: {result:?}"
        );
    }
    Ok(())
}
