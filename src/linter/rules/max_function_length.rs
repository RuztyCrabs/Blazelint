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

            let line_count = function_source
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with("//"))
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
