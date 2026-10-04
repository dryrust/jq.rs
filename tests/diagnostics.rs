// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use jq::{CompilationDiagnostic, CompilationPhase, JsonFilterError};

#[cfg(feature = "jaq")]
fn diagnostics(source: &str) -> Vec<CompilationDiagnostic> {
    match source.parse::<jq::JsonFilter>() {
        Err(JsonFilterError::Compile(diagnostics)) => diagnostics,
        _ => panic!("expected a compilation failure"),
    }
}

#[cfg(feature = "jaq")]
#[test]
fn lexer_diagnostics_report_utf8_byte_spans() {
    for (source, span, expected) in [
        ("[", 1..1, "closing bracket"),
        ("$", 1..1, "identifier"),
        (r#""\q""#, 2..3, "string escape sequence"),
        ("\"é\" | § # private source", 7..9, "token"),
    ] {
        let diagnostics = diagnostics(source);
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.phase, CompilationPhase::Lex);
        assert_eq!(diagnostic.span, Some(span));
        assert_eq!(diagnostic.message, format!("expected {expected}"));
        assert!(!diagnostic.to_string().contains("private source"));
    }
}

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

#[cfg(feature = "jaq")]
#[test]
fn parser_diagnostics_locate_missing_and_unexpected_tokens() {
    for (source, span, message) in [
        ("1 +", 3..3, "expected expression"),
        ("if 0", 4..4, "expected then"),
        ("[1,]", 3..4, "expected expression"),
        ("0;", 1..2, "expected end of input"),
        ("\"é\"\n| 1 +", 10..10, "expected expression"),
    ] {
        let diagnostics = diagnostics(source);
        assert_eq!(diagnostics.len(), 1, "{source}: {diagnostics:?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.phase, CompilationPhase::Parse);
        assert_eq!(diagnostic.span, Some(span));
        assert_eq!(diagnostic.message, message);
    }
}
