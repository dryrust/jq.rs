// This is free and unencumbered software released into the public domain.

#![cfg(all(feature = "jaq", feature = "std"))]

use core::sync::atomic::{AtomicUsize, Ordering};
use jq::{JsonFilter, JsonFilterError};
use serde_json::json;

static RECORDS: AtomicUsize = AtomicUsize::new(0);

struct Counter;

impl log::Log for Counter {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }

    fn log(&self, _: &log::Record) {
        RECORDS.fetch_add(1, Ordering::SeqCst);
    }

    fn flush(&self) {}
}

#[test]
fn stderr_and_debug_log_only_on_opt_in() -> Result<(), JsonFilterError> {
    log::set_logger(&Counter).unwrap();
    log::set_max_level(log::LevelFilter::Trace);
    let filter: JsonFilter = r#"stderr | debug | debug("message")"#.parse()?;
    assert_eq!(filter.filter_json_all(json!(1))?, [json!(1)]);
    let expected = if cfg!(feature = "log") { 3 } else { 0 };
    assert_eq!(RECORDS.load(Ordering::SeqCst), expected);
    Ok(())
}
