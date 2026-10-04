// This is free and unencumbered software released into the public domain.

use core::hint::black_box;
use jq::JsonFilter;
use serde_json::json;
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
        jaq_std::defs().chain(jaq_json::defs()).collect::<Vec<_>>()
    });
    measure("stdlib/native-functions", iterations, || {
        jaq_std::funs::<jaq_json::Val>()
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
}
