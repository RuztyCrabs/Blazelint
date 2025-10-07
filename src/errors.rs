//! Shared diagnostic structures used by the lexer, parser, and front-end.
//!
//! The linter does not yet surface formatted diagnostics to users, but the
//! types in this module allow each stage of the pipeline to report failures in
//! a structured way, retaining byte spans and auxiliary notes.

use std::ops::Range;

/// Byte range within the original source file.
pub type Span = Range<usize>;

/// General classification for diagnostics emitted by the linter.
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticKind {
    Lex,
    Parse,
    Semantic,
}

/// Structured diagnostic message produced by either the lexer or parser.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
}

impl Diagnostic {
    /// Creates a new diagnostic with the provided message and span.
    pub fn new(kind: DiagnosticKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            message: message.into(),
            span,
            notes: Vec::new(),
        }
    }

    /// Attaches an additional note to the diagnostic, returning the mutated value.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }
}

/// Error emitted when the lexer fails to tokenise the input stream.
#[derive(Debug, Clone)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl LexError {
    /// Creates a new lexical error for the given span.
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }
}

impl From<LexError> for Diagnostic {
    fn from(err: LexError) -> Self {
        Diagnostic::new(DiagnosticKind::Lex, err.message, err.span)
    }
}

/// Error produced while parsing the token stream into an AST.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub expected: Option<&'static str>,
}

#[allow(dead_code)]
impl ParseError {
    /// Creates a new parse error with an optional expectation hint.
    pub fn new(message: impl Into<String>, span: Span, expected: Option<&'static str>) -> Self {
        Self {
            message: message.into(),
            span,
            expected,
        }
    }
}

impl From<ParseError> for Diagnostic {
    fn from(err: ParseError) -> Self {
        let mut diagnostic = Diagnostic::new(DiagnosticKind::Parse, err.message, err.span);
        if let Some(expected) = err.expected {
            diagnostic = diagnostic.with_note(format!("expected: {expected}"));
        }
        diagnostic
    }
}
