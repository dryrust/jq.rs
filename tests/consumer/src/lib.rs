//! Consumer feature-unification regressions for jq.
#![no_std]

extern crate alloc;

/// Instantiates the public API for targets without a standard library.
#[cfg(any(feature = "jaq", feature = "jq"))]
pub fn exercise_api() -> Result<alloc::vec::Vec<serde_json::Value>, jq::JsonFilterError> {
    use core::ops::ControlFlow;
    use jq::JsonFilter;
    use serde_json::json;

    let identity: JsonFilter = ".".parse()?;
    let main = identity.filter_json_str("1")?;
    let mut filter = JsonFilter::with_bindings("., $offset, inputs", [("offset", json!(0))])?;
    filter.set_bindings([("offset", json!(2))])?;
    let mut values = alloc::vec::Vec::new();
    let _ = filter
        .clone()
        .filter_json_visit_with_inputs(main, [Ok(json!(3))], |value| {
            values.push(value);
            ControlFlow::<()>::Continue(())
        })?;
    Ok(values)
}
