//! Rule to enforce a maximum function length.

use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
};

const DEFAULT_MAX_FUNCTION_LENGTH: usize = 50;

/// A rule that enforces a maximum function length.
pub struct MaxFunctionLengthRule {
    max_length: usize,
}

impl MaxFunctionLengthRule {
    pub fn new() -> Self {
        Self {
            max_length: DEFAULT_MAX_FUNCTION_LENGTH,
        }
    }
}

impl LintRule for MaxFunctionLengthRule {
    fn name(&self) -> &'static str {
        "max_function_length"
    }

    fn description(&self) -> &'static str {
        "Enforces a maximum function length."
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn check(&self, ast: &[Stmt], _file_path: &str, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for stmt in ast {
            if let Stmt::Function { name, span, .. } = stmt {
                // Count the number of lines in the function source
                let function_source = &source[span.start..span.end];
                let line_count = function_source.lines().count();
                if line_count > self.max_length {
                    diagnostics.push(Diagnostic::new_with_severity(
                        DiagnosticKind::Linter,
                        self.severity(),
                        format!(
                            "Function \"{}\" has {} lines (exceeds maximum of {})",
                            name, line_count, self.max_length
                        ),
                        span.clone(),
                    ));
                }
            }
        }
        diagnostics
    }
}
