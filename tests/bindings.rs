// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

#[test]
fn bindings_are_owned_json_values_in_declared_order() -> Result<(), JsonFilterError> {
    let filter = JsonFilter::with_bindings(
        "[$second, $first, .]",
        [
            ("first", json!({"data": [1, false, null]})),
            ("second", json!("日本語 \" | error(1)")),
        ],
    )?;
    for filter in [filter.clone(), filter] {
        for input in [json!(0), json!("next")] {
            let expected = json!(["日本語 \" | error(1)", {"data": [1, false, null]}, input]);
            assert_eq!(filter.filter_json(input.clone())?, expected);
            assert_eq!(filter.filter_json_str(input.to_string())?, expected);
            assert_eq!(
                filter.filter_json_all(input.clone())?,
                core::slice::from_ref(&expected)
            );
            assert_eq!(filter.filter_json_str_all(input.to_string())?, [expected]);
        }
    }
    Ok(())
}

#[test]
fn invalid_and_duplicate_binding_names_are_rejected() {
    for name in ["", "$x", "1x", "a-b", "x | halt", "日本語", "ARGS", "ENV"] {
        assert!(
            matches!(
                JsonFilter::with_bindings(".", [(name, json!(null))]),
                Err(JsonFilterError::Bindings(_))
            ),
            "{name}"
        );
    }
    assert!(matches!(
        JsonFilter::with_bindings("$x", [("x", json!(1)), ("x", json!(2))]),
        Err(JsonFilterError::Bindings(_))
    ));
    assert!(matches!(
        JsonFilter::with_bindings("$missing", [("other", json!(1))]),
        Err(JsonFilterError::Compile(_))
    ));
}

#[test]
fn bindings_support_large_values_and_local_shadowing() -> Result<(), JsonFilterError> {
    let value = json!("x".repeat(256 * 1024));
    let filter = JsonFilter::with_bindings("$value", [("value", value.clone())])?;
    assert_eq!(filter.filter_json(json!(null))?, value);
    let filter = JsonFilter::with_bindings("1 as $_x2 | $_x2", [("_x2", json!(9))])?;
    assert_eq!(filter.filter_json(json!(null))?, json!(1));
    Ok(())
}

#[test]
fn rebinding_preserves_declaration_order_and_clone_independence() -> Result<(), JsonFilterError> {
    let mut filter = JsonFilter::with_bindings(
        "[$left, $right, .]",
        [("left", json!(1)), ("right", json!(2))],
    )?;
    let cloned = filter.clone();
    for left in [json!({"updated": true}), json!("日本語")] {
        filter.set_bindings([("right", json!(null)), ("left", left.clone())])?;
        assert_eq!(filter.filter_json(json!(3))?, json!([left, null, 3]));
        assert_eq!(filter.filter_json_str_all("4")?, [json!([left, null, 4])]);
        assert_eq!(cloned.filter_json(json!(5))?, json!([1, 2, 5]));
    }
    Ok(())
}

#[test]
fn failed_rebinding_keeps_all_previous_values() -> Result<(), JsonFilterError> {
    let mut filter = JsonFilter::with_bindings("[$a, $b]", [("a", json!(1)), ("b", json!(2))])?;
    for bindings in [
        vec![],
        vec![("a", json!(9))],
        vec![("a", json!(9)), ("unknown", json!(10))],
        vec![("a", json!(9)), ("a", json!(10))],
        vec![("a", json!(9)), ("$b", json!(10))],
        vec![("a", json!(9)), ("b", json!(10)), ("extra", json!(11))],
    ] {
        assert!(matches!(
            filter.set_bindings(bindings),
            Err(JsonFilterError::Bindings(_))
        ));
        assert_eq!(filter.filter_json(json!(null))?, json!([1, 2]));
    }
    let mut identity = JsonFilter::default();
    identity.set_bindings(core::iter::empty::<(&str, serde_json::Value)>())?;
    assert!(matches!(
        identity.set_bindings([("new", json!(1))]),
        Err(JsonFilterError::Bindings(_))
    ));
    assert_eq!(identity.filter_json(json!(42))?, json!(42));
    Ok(())
}
