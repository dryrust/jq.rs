// This is free and unencumbered software released into the public domain.

use crate::{CompilationDiagnostic, diagnostic::DisplayDiagnostics};
use alloc::{string::String, vec::Vec};

/// A failure to parse JSON, compile a filter, or evaluate it.
#[derive(Debug, thiserror::Error)]
pub enum JsonFilterError {
    /// Variable names or values do not match the filter's binding contract.
    #[error("binding error: {0}")]
    Bindings(String),

    /// The input string is not exactly one valid JSON value.
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// A filter result cannot be represented as a JSON value.
    #[error("output conversion error: {0}")]
    Output(#[source] serde_json::Error),

    /// The filter program could not be loaded or compiled.
    #[error("compilation error: {}", DisplayDiagnostics(.0))]
    Compile(Vec<CompilationDiagnostic>),

    /// A single-result method evaluated a filter that produced no values.
    #[error("no output")]
    NoOutput,

    /// Evaluation failed with an owned, backend-independent diagnostic.
    #[error("execution error: {0}")]
    Execute(String),
}
