// This is free and unencumbered software released into the public domain.

#![cfg(any(feature = "jaq", feature = "jq"))]

use jq::{CompilationDiagnostic, CompilationPhase, JsonFilterError};

fn diagnostics(source: &str) -> Vec<CompilationDiagnostic> {
    match source.parse::<jq::JsonFilter>() {
        Err(JsonFilterError::Compile(diagnostics)) => diagnostics,
        _ => panic!("expected a compilation failure"),
    }
}

#[cfg(all(feature = "jq", not(feature = "jaq")))]
#[test]
fn upstream_diagnostics_omit_source_excerpts() {
    for source in ["[", "1 +", "unknown", "$unbound"] {
        let diagnostics = diagnostics(&format!("{source} # PRIVATE_SOURCE_MARKER"));
        assert!(!diagnostics.is_empty());
        for diagnostic in diagnostics {
            assert_eq!(diagnostic.phase, CompilationPhase::Compile);
            assert!(diagnostic.span.is_none());
            assert!(!diagnostic.message.is_empty());
            assert!(!diagnostic.message.contains("PRIVATE_SOURCE_MARKER"));
            assert!(!diagnostic.message.contains("<top-level>"));
        }
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

#[cfg(feature = "jaq")]
#[test]
fn symbol_diagnostics_identify_the_offending_reference() {
    for (source, symbol, message) in [
        ("unknown(1; 2)", "unknown", "undefined function unknown/2"),
        ("$unbound", "$unbound", "undefined variable $unbound"),
        ("break $label", "$label", "undefined label $label"),
        (
            "# unknown\n\"é\" | unknown",
            "unknown",
            "undefined function unknown/0",
        ),
    ] {
        let diagnostics = diagnostics(source);
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        let offset = source.rfind(symbol).unwrap();
        assert_eq!(diagnostic.phase, CompilationPhase::Compile);
        assert_eq!(diagnostic.span, Some(offset..offset + symbol.len()));
        assert_eq!(diagnostic.message, message);
    }
}

#[cfg(feature = "jaq")]
#[test]
fn symbol_errors_remain_available_after_source_is_dropped() {
    let diagnostics = {
        let source = String::from("[$a, missing]");
        diagnostics(&source)
    };
    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message == "undefined variable $a")
    );
    assert!(
        diagnostics
            .iter()
            .any(|d| d.message == "undefined function missing/0")
    );
}

#[cfg(feature = "jaq")]
#[test]
fn module_and_data_imports_report_loading_diagnostics() {
    for (source, reason) in [
        (
            r#"include "missing"; . # private source"#,
            "module loading not supported",
        ),
        (
            r#"import "missing" as m; m::value"#,
            "module loading not supported",
        ),
        (
            r#"import "missing" as $data; $data"#,
            "data imports are not supported",
        ),
    ] {
        let diagnostics = diagnostics(source);
        assert_eq!(diagnostics.len(), 1);
        let diagnostic = &diagnostics[0];
        let start = source.find("missing").unwrap();
        assert_eq!(diagnostic.phase, CompilationPhase::Load);
        assert_eq!(diagnostic.span, Some(start..start + "missing".len()));
        assert_eq!(diagnostic.message, format!("missing: {reason}"));
        assert!(!diagnostic.to_string().contains("private source"));
    }
}
