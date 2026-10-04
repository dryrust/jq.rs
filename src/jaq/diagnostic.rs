// This is free and unencumbered software released into the public domain.

use crate::{CompilationDiagnostic, CompilationPhase};
use alloc::{format, vec, vec::Vec};
use jaq_core::load;

pub(super) fn load_errors(errors: load::Errors<&str, ()>) -> Vec<CompilationDiagnostic> {
    errors
        .into_iter()
        .flat_map(|(file, error)| match error {
            load::Error::Lex(errors) => errors
                .into_iter()
                .map(|(expected, found)| {
                    // Lexers return a suffix at the error, not a single token. Highlight
                    // only its first character, or the empty position at end of input.
                    let length = found.chars().next().map_or(0, char::len_utf8);
                    CompilationDiagnostic {
                        phase: CompilationPhase::Lex,
                        message: format!("expected {}", expected.as_str()),
                        span: Some(load::span(file.code, &found[..length])),
                    }
                })
                .collect(),
            load::Error::Parse(errors) => errors
                .into_iter()
                .map(|(expected, found)| CompilationDiagnostic {
                    phase: CompilationPhase::Parse,
                    message: format!(
                        "expected {}",
                        match expected {
                            load::parse::Expect::Term => "expression",
                            load::parse::Expect::Nothing => "end of input",
                            _ => expected.as_str(),
                        }
                    ),
                    span: Some(load::span(file.code, found)),
                })
                .collect(),
            error => vec![CompilationDiagnostic {
                phase: CompilationPhase::Load,
                message: format!("{error:?}"),
                span: None,
            }],
        })
        .collect()
}
