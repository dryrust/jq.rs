// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

#[test]
fn bare_metal_consumer_has_a_working_host_contract() -> Result<(), jq::JsonFilterError> {
    assert_eq!(
        jq_consumer_tests::exercise_api()?,
        [
            serde_json::json!(1),
            serde_json::json!(2),
            serde_json::json!(3)
        ]
    );
    Ok(())
}
