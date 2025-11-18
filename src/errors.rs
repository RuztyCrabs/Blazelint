//! Shared diagnostic structures used by the lexer, parser, and front-end.
//!
//! The linter does not yet surface formatted diagnostics to users, but the
//! types in this module allow each stage of the pipeline to report failures in
//! a structured way, retaining byte spans and auxiliary notes.

use std::ops::Range;

/// Byte range within the original source file.
pub type Span = Range<usize>;

/// Represents the severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// General classification for diagnostics emitted by the linter.
#[derive(Debug, Clone, Copy)]
pub enum DiagnosticKind {
    Lex,
    Parse,
    Semantic,
    Linter,
}

/// Represents a position in the source code.
#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

/// Structured diagnostic message produced by either the lexer or parser.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub kind: DiagnosticKind,
    pub severity: Severity, // New field for severity
    pub message: String,
    pub span: Span,
    pub notes: Vec<String>,
    pub position: Option<Position>,
}

impl Diagnostic {
    /// Creates a new diagnostic with the provided message and span, defaulting to Error severity.
    pub fn new(kind: DiagnosticKind, message: impl Into<String>, span: Span) -> Self {
        Self {
            kind,
            severity: Severity::Error, // Default to Error
            message: message.into(),
            span,
            notes: Vec::new(),
            position: None,
        }
    }

    /// Creates a new diagnostic with the provided message, span, and explicit severity.
    pub fn new_with_severity(
        kind: DiagnosticKind,
        severity: Severity,
        message: impl Into<String>,
        span: Span,
    ) -> Self {
        Self {
            kind,
            severity,
            message: message.into(),
            span,
            notes: Vec::new(),
            position: None,
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
        Diagnostic::new_with_severity(DiagnosticKind::Lex, Severity::Error, err.message, err.span)
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
        let mut diagnostic = Diagnostic::new_with_severity(
            DiagnosticKind::Parse,
            Severity::Error,
            err.message,
            err.span,
        );
        if let Some(expected) = err.expected {
            diagnostic = diagnostic.with_note(format!("expected: {expected}"));
        }
        diagnostic
    }
}
