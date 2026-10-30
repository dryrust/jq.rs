// This is free and unencumbered software released into the public domain.

#![cfg(all(feature = "jaq", feature = "stacker"))]

use jq::{JsonFilter, JsonFilterError};
use serde_json::{Value, json};

// Deep programs and values used to overflow small thread stacks, such as
// those of async runtime workers, aborting the whole process. Each case
// exceeds the old limit on a 2 MiB stack by several times, even in debug.
#[test]
fn deep_programs_and_values_do_not_overflow_small_stacks() {
    let deep = 5_000;
    let cases = [
        (
            format!("{}{} | length", "[".repeat(deep), "]".repeat(deep)),
            json!(1),
        ),
        (
            format!("def f: if . == {deep} then 0 else (. + 1 | f) + 1 end; 0 | f"),
            json!(deep),
        ),
        (
            format!("reduce range({deep}) as $i (null; [.]) | 1"),
            json!(1),
        ),
        (
            format!("reduce range({deep}) as $i (null; [.]) | tojson | length"),
            json!(deep * 2 + "null".len()),
        ),
    ];
    std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            for (source, expected) in cases {
                let filter: JsonFilter = source.parse()?;
                assert_eq!(filter.filter_json(Value::Null)?, expected);
            }
            Ok::<_, JsonFilterError>(())
        })
        .unwrap()
        .join()
        .unwrap()
        .unwrap();
}
