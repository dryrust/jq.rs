// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use jq::{CompilationDiagnostic, CompilationPhase, JsonFilterError};

#[test]
fn diagnostic_display_is_readable_and_owned() {
    let diagnostic = CompilationDiagnostic {
        phase: CompilationPhase::Parse,
        message: "expected expression".into(),
        span: Some(4..4),
    };
    assert_eq!(
        diagnostic.to_string(),
        "parsing: expected expression at bytes 4..4"
    );
    let error = JsonFilterError::Compile(vec![
        diagnostic,
        CompilationDiagnostic {
            phase: CompilationPhase::Compile,
            message: "unknown function".into(),
            span: None,
        },
    ]);
    assert_eq!(
        error.to_string(),
        "compilation error: parsing: expected expression at bytes 4..4; compilation: unknown function"
    );
}
