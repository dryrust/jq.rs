// This is free and unencumbered software released into the public domain.

use core::{hint::black_box, ops::ControlFlow};
use jq::JsonFilter;
use serde_json::{Value, json};
use std::time::Instant;

fn measure<T>(name: &str, iterations: u32, mut run: impl FnMut() -> T) {
    for _ in 0..10 {
        black_box(run());
    }
    let start = Instant::now();
    for _ in 0..iterations {
        black_box(run());
    }
    println!("{name}: {:?}/iteration", start.elapsed() / iterations);
}

fn main() {
    let iterations = std::env::var("JQ_BENCH_ITERS")
        .map(|value| value.parse::<u32>().expect("JQ_BENCH_ITERS must be a u32"))
        .unwrap_or(1_000);
    assert!(iterations > 0, "JQ_BENCH_ITERS must be positive");
    println!("jaq backend; {iterations} iterations; 10 warmup iterations");

    measure("stdlib/definitions", iterations, || {
        jaq_core::defs()
            .chain(jaq_std::defs())
            .chain(jaq_json::defs())
            .collect::<Vec<_>>()
    });
    measure("stdlib/native-functions", iterations, || {
        jaq_std::funs::<jaq_core::data::JustLut<jaq_json::Val>>()
            .chain(jaq_json::funs())
            .collect::<Vec<_>>()
    });
    measure("compile/identity", iterations, || {
        black_box(".").parse::<JsonFilter>().unwrap()
    });

    let source = ".users[] | select(.active) | .profile.name";
    let input = json!({"users": [{"active": true, "profile": {"name": "Ada"}}]});
    let filter: JsonFilter = source.parse().unwrap();
    assert_eq!(filter.filter_json(input.clone()).unwrap(), json!("Ada"));
    measure("compile/nested-selection", iterations, || {
        black_box(source).parse::<JsonFilter>().unwrap()
    });
    measure("reuse/compiled", iterations, || {
        filter.filter_json(black_box(input.clone())).unwrap()
    });
    measure("reuse/compile-per-call", iterations, || {
        black_box(source)
            .parse::<JsonFilter>()
            .unwrap()
            .filter_json(black_box(input.clone()))
            .unwrap()
    });

    let large = Value::Array(
        (0..1_024)
            .map(|id| json!({"id": id, "profile": {"name": "日本語 🦀", "tags": [1, 2, 3]}}))
            .collect(),
    );
    for (size, input) in [("small", input), ("large-nested", large.clone())] {
        let text = input.to_string();
        let identity = JsonFilter::default();
        assert_eq!(identity.filter_json_str(&text).unwrap(), input);
        measure(
            &format!("conversion/{size}/clone-value"),
            iterations,
            || black_box(&input).clone(),
        );
        measure(&format!("conversion/{size}/parse-json"), iterations, || {
            serde_json::from_str::<Value>(black_box(&text)).unwrap()
        });
        measure(&format!("conversion/{size}/parse-jaq"), iterations, || {
            jaq_json::read::parse_single(black_box(text.as_bytes())).unwrap()
        });
        measure(&format!("identity/{size}/value"), iterations, || {
            identity.filter_json(black_box(input.clone())).unwrap()
        });
        measure(&format!("identity/{size}/string"), iterations, || {
            identity.filter_json_str(black_box(&text)).unwrap()
        });
    }

    let elements: JsonFilter = ".[]".parse().unwrap();
    for (size, input) in [("small", json!([1, 2, 3, 4])), ("large-nested", large)] {
        assert_eq!(elements.filter_json(input.clone()).unwrap(), input[0]);
        assert_eq!(
            elements.filter_json_all(input.clone()).unwrap(),
            *input.as_array().unwrap()
        );
        measure(&format!("outputs/{size}/first"), iterations, || {
            elements.filter_json(black_box(input.clone())).unwrap()
        });
        measure(&format!("outputs/{size}/collect"), iterations, || {
            elements.filter_json_all(black_box(input.clone())).unwrap()
        });
        measure(&format!("outputs/{size}/visit-all"), iterations, || {
            elements
                .filter_json_visit(black_box(input.clone()), |value| {
                    black_box(value);
                    ControlFlow::<()>::Continue(())
                })
                .unwrap()
        });
        measure(&format!("outputs/{size}/visit-first"), iterations, || {
            elements
                .filter_json_visit(black_box(input.clone()), ControlFlow::Break)
                .unwrap()
        });
    }
}
