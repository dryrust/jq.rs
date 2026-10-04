// This is free and unencumbered software released into the public domain.

use alloc::string::String;
use core::{fmt, ops::Range};

/// The phase in which a program failed to compile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompilationPhase {
    /// Loading the compiler or a referenced module.
    Load,
    /// Recognizing tokens in the program source.
    Lex,
    /// Parsing tokens into a program.
    Parse,
    /// Resolving symbols or compiling the parsed program.
    Compile,
}

impl fmt::Display for CompilationPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Load => "loading",
            Self::Lex => "lexing",
            Self::Parse => "parsing",
            Self::Compile => "compilation",
        })
    }
}

/// An owned, backend-independent description of a compilation failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilationDiagnostic {
    /// The phase that reported the failure.
    pub phase: CompilationPhase,
    /// A human-readable message, without a complete source-code dump.
    pub message: String,
    /// A half-open UTF-8 byte range in the supplied program, when available.
    ///
    /// An empty range denotes a position such as end of input. Backends that
    /// cannot reliably report source locations leave this as `None`.
    pub span: Option<Range<usize>>,
}

impl fmt::Display for CompilationDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.phase, self.message)?;
        if let Some(span) = &self.span {
            write!(f, " at bytes {}..{}", span.start, span.end)?;
        }
        Ok(())
    }
}

pub(crate) struct DisplayDiagnostics<'a>(pub &'a [CompilationDiagnostic]);

impl fmt::Display for DisplayDiagnostics<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, diagnostic) in self.0.iter().enumerate() {
            if index != 0 {
                f.write_str("; ")?;
            }
            diagnostic.fmt(f)?;
        }
        Ok(())
    }
}
