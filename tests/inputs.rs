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

#[test]
fn auxiliary_exhaustion_distinguishes_input_from_inputs() -> Result<(), JsonFilterError> {
    let empty = || core::iter::empty::<Result<serde_json::Value, String>>();
    let filter: JsonFilter = "inputs".parse()?;
    assert!(
        filter
            .filter_json_all_with_inputs(json!(0), empty())?
            .is_empty()
    );
    assert!(matches!(
        filter.filter_json_with_inputs(json!(0), empty()),
        Err(JsonFilterError::NoOutput)
    ));
    let filter: JsonFilter = "input".parse()?;
    assert!(matches!(
        filter.filter_json_with_inputs(json!(0), empty()),
        Err(JsonFilterError::Execute(_))
    ));
    assert!(matches!(
        filter.filter_json_all_with_inputs(json!(0), empty()),
        Err(JsonFilterError::Execute(_))
    ));
    Ok(())
}

#[test]
fn input_errors_follow_earlier_results_and_are_catchable() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "inputs".parse()?;
    let auxiliary = || [Ok(json!(1)), Err(String::from("read failed")), Ok(json!(3))];
    assert_eq!(
        filter.filter_json_with_inputs(json!(0), auxiliary())?,
        json!(1)
    );
    let error = filter
        .filter_json_all_with_inputs(json!(0), auxiliary())
        .unwrap_err();
    assert!(matches!(error, JsonFilterError::Execute(_)));
    assert!(error.to_string().contains("read failed"));
    let mut seen = Vec::new();
    let result = filter.filter_json_visit_with_inputs(json!(0), auxiliary(), |value| {
        seen.push(value);
        ControlFlow::<()>::Continue(())
    });
    assert!(matches!(result, Err(JsonFilterError::Execute(_))));
    assert_eq!(seen, [json!(1)]);

    let filter: JsonFilter = "[(try input catch .), input]".parse()?;
    assert_eq!(
        filter
            .filter_json_with_inputs(json!(0), [Err(String::from("read failed")), Ok(json!(2))])?,
        json!(["read failed", 2])
    );
    Ok(())
}

#[test]
fn unused_values_and_errors_do_not_restart_evaluation() -> Result<(), JsonFilterError> {
    let filter = JsonFilter::default();
    assert_eq!(
        filter
            .filter_json_all_with_inputs(json!(42), [Err(String::from("unused")), Ok(json!(99))])?,
        [json!(42)]
    );
    let filter: JsonFilter = "empty".parse()?;
    assert!(
        filter
            .filter_json_all_with_inputs(json!(42), [Err(String::from("unused"))])?
            .is_empty()
    );
    Ok(())
}

#[test]
fn auxiliary_values_cannot_be_confused_with_transport_errors() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "inputs".parse()?;
    let values = [
        json!([false, "literal"]),
        json!({"error": "literal", "value": null}),
        json!(null),
    ];
    assert_eq!(
        filter.filter_json_all_with_inputs(json!(0), values.iter().cloned().map(Ok))?,
        values
    );
    Ok(())
}

#[test]
fn infinite_auxiliary_streams_allow_early_stopping() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "inputs".parse()?;
    let mut count = 0;
    let status = filter.filter_json_visit_with_inputs(
        json!(0),
        core::iter::repeat_with(|| Ok(json!(1))),
        |_| {
            count += 1;
            if count == 5 {
                ControlFlow::Break(count)
            } else {
                ControlFlow::Continue(())
            }
        },
    )?;
    assert_eq!(status, ControlFlow::Break(5));
    let filter: JsonFilter = "input".parse()?;
    assert_eq!(
        filter.filter_json_all_with_inputs(json!(0), core::iter::repeat_with(|| Ok(json!(1))))?,
        [json!(1)]
    );
    Ok(())
}

#[test]
fn auxiliary_visitors_can_capture_non_send_state() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "inputs".parse()?;
    let seen = std::rc::Rc::new(core::cell::RefCell::new(Vec::new()));
    let captured = seen.clone();
    let status = filter.filter_json_visit_with_inputs(json!(0), [Ok(json!(1))], move |value| {
        captured.borrow_mut().push(value);
        ControlFlow::<()>::Continue(())
    })?;
    assert_eq!(status, ControlFlow::Continue(()));
    assert_eq!(*seen.borrow(), [json!(1)]);
    Ok(())
}

#[test]
fn visitor_panic_releases_auxiliary_resources() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "inputs".parse()?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        filter.filter_json_visit_with_inputs(
            json!(0),
            core::iter::repeat_with(|| Ok(json!(1))),
            |_| -> ControlFlow<()> {
                panic!("visitor failure");
            },
        )
    }));
    assert!(result.is_err());
    assert_eq!(
        filter.filter_json_with_inputs(json!(0), [Ok(json!(2))])?,
        json!(2)
    );
    Ok(())
}

#[test]
fn string_methods_still_require_one_main_json_value() -> Result<(), JsonFilterError> {
    let filter: JsonFilter = "input".parse()?;
    for input in ["", "1 2", "{} []"] {
        assert!(matches!(
            filter.filter_json_str(input),
            Err(JsonFilterError::Parse(_))
        ));
        assert!(matches!(
            filter.filter_json_str_all(input),
            Err(JsonFilterError::Parse(_))
        ));
    }
    Ok(())
}
