// This is free and unencumbered software released into the public domain.

//! Compile and reuse jq-style filters over JSON values.
//!
//! The default `jaq` feature provides the filtering API. The crate uses `alloc`
//! and is declared `no_std`, but the current jaq dependency graph still requires
//! a target with the standard library. The `std` feature is enabled by default.
//!
//! # Features
//!
//! - `all`: enables every implemented backend (`jaq` and `jq`).
//! - `jaq`: enables the jaq implementation and public filtering API.
//! - `std`: enables standard-library support in dependencies.
//! - `jq`: enables the upstream jq subprocess backend and requires `std`.
//! - `libjq`, `xq`: reserved for unimplemented backends.
//! - `unstable`: reserved for experimental APIs; currently has no effect.
//!
//! Defaults enable `all` and `std`. Reserved flags do not provide a filtering
//! API on their own. `jaq` takes precedence when both backends are enabled.
//! To use upstream jq, disable defaults and enable only `jq`; a `jq` executable
//! must be available on `PATH` during construction and evaluation. Each call
//! starts a fresh process, which is reaped on completion or early stopping.
//! Unlike jaq, the subprocess backend validates source at construction and
//! recompiles it within each evaluation process.
//!
//! ```
//! # #[cfg(any(feature = "jaq", feature = "jq"))] {
//! use jq::JsonFilter;
//! use serde_json::json;
//!
//! let filter: JsonFilter = ".items[] | .name".parse()?;
//! assert_eq!(
//!     filter.filter_json_all(json!({"items": [{"name": "Ada"}]}))?,
//!     [json!("Ada")],
//! );
//! assert_eq!(filter.filter_json_str(r#"{"items":[{"name":"Lin"}]}"#)?,
//!            json!("Lin"));
//! # }
//! # Ok::<(), std::boxed::Box<dyn core::error::Error>>(())
//! ```
//!
//! Default filter construction is identity (`.`). Parsing a filter program
//! reports compilation errors; string-based filtering methods separately
//! report invalid JSON input. Single-result methods evaluate only the first
//! result, while collection methods detect later execution errors as well.
//!
//! # Backend compatibility
//!
//! The default backend is jaq, whose language behavior can differ from upstream
//! jq. jaq rejects external module and data imports. Named JSON variables can be
//! supplied with `JsonFilter::with_bindings`. Auxiliary input is empty:
//! `inputs` produces no values and `input` encounters exhaustion. Backend
//! selection is intended to remain an implementation detail of the public API.
//!
//! # Numbers
//!
//! The following describes jaq; the subprocess backend follows the installed
//! upstream jq version's numeric semantics.
//!
//! jaq's nonfinite floating-point results (`nan`, `infinite`, `1 / 0`) become
//! JSON null, including inside arrays and objects. Literal numbers are converted
//! using `serde_json::Number`: integers beyond its exact range may lose precision,
//! and out-of-range exponents such as `1e400` return an output conversion error.
//! A consumer enabling `serde_json/arbitrary_precision` can retain such literals
//! exactly. This does not make backend arithmetic arbitrary-precision; jaq uses
//! machine-sized integers and floating-point arithmetic internally.
//!
//! # Object order
//!
//! Input and output objects use `serde_json::Map` ordering. By default, input
//! keys are sorted, so `.[]` on `{"z":1,"a":2}` yields `2, 1` and
//! `keys_unsorted` yields `["a", "z"]`. Consumers can enable
//! `serde_json/preserve_order` to retain insertion order instead (`1, 2` and
//! `["z", "a"]`); that dependency feature requires the standard library.
//! This applies to both string and `Value` inputs, including nested objects.
//! Cargo unifies dependency features, so another dependency can enable this
//! behavior too. Objects constructed within a jaq program retain the backend's
//! insertion order until converted to output JSON; `keys` explicitly sorts keys.

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

#[cfg(any(feature = "jaq", feature = "jq"))]
mod diagnostic;
#[cfg(any(feature = "jaq", feature = "jq"))]
pub use diagnostic::{CompilationDiagnostic, CompilationPhase};

#[cfg(any(feature = "jaq", feature = "jq"))]
mod error;
#[cfg(any(feature = "jaq", feature = "jq"))]
pub use error::JsonFilterError;

#[cfg(any(feature = "jaq", feature = "jq"))]
mod filter;
#[cfg(feature = "jaq")]
mod jaq;
#[cfg(all(feature = "jq", not(feature = "jaq")))]
mod jq;
#[cfg(any(feature = "jaq", feature = "jq"))]
pub use filter::JsonFilter;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
