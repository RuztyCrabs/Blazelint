//! Rule to enforce a maximum function length.

use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind},
    linter::Rule,
};

const DEFAULT_MAX_FUNCTION_LENGTH: usize = 50;

/// A rule that enforces a maximum function length.
pub struct MaxFunctionLength {
    max_length: usize,
}

impl MaxFunctionLength {
    pub fn new(max_length: Option<usize>) -> Self {
        Self {
            max_length: max_length.unwrap_or(DEFAULT_MAX_FUNCTION_LENGTH),
        }
    }
}

impl Rule for MaxFunctionLength {
    fn name(&self) -> &'static str {
        "max-function-length"
    }

    fn description(&self) -> &'static str {
        "Enforces a maximum function length."
    }

    fn validate(&self, statement: &Stmt, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Stmt::Function {
            body, name, span, ..
        } = statement
        {
            if body.is_empty() {
                return diagnostics;
            }

            let function_source = &source[span.start..span.end];

            // Remove block comments before counting lines
            let mut without_block_comments = String::new();
            let mut in_block_comment = false;
            let mut chars = function_source.chars().peekable();

            while let Some(c) = chars.next() {
                if in_block_comment {
                    if c == '*' && chars.peek() == Some(&'/') {
                        chars.next(); // consume '/'
                        in_block_comment = false;
                    }
                } else if c == '/' && chars.peek() == Some(&'*') {
                    chars.next(); // consume '*'
                    in_block_comment = true;
                } else {
                    without_block_comments.push(c);
                }
            }

            let line_count = without_block_comments
                .lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("//"))
                .filter(|line| !line.is_empty())
                .count();

            if line_count > self.max_length {
                diagnostics.push(Diagnostic::new(
                    DiagnosticKind::Linter,
                    format!(
                        "Function \"{}\" has {} lines (exceeds maximum of {})",
                        name, line_count, self.max_length
                    ),
                    span.clone(),
                ));
            }
        }

        diagnostics
    }
}
