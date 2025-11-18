//! Rule to enforce a maximum function length.

use crate::{
    ast::Stmt,
    config::Config,
    errors::{Diagnostic, DiagnosticKind},
    linter::registry::LintRule,
};

/// A rule that enforces a maximum function length.
pub struct MaxFunctionLengthRule;

impl Default for MaxFunctionLengthRule {
    fn default() -> Self {
        Self::new()
    }
}

impl MaxFunctionLengthRule {
    pub fn new() -> Self {
        Self
    }
}

impl LintRule for MaxFunctionLengthRule {
    fn name(&self) -> &'static str {
        "max-function-length"
    }

    fn description(&self) -> &'static str {
        "Enforces a maximum function length."
    }

    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        source: &str,
        config: &Config,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let max_length = config.settings.max_function_length as usize;
        let severity = self.severity(config);
        for stmt in ast {
            if let Stmt::Function { name, span, .. } = stmt {
                // Count the number of lines in the function source
                let function_source = &source[span.start..span.end];
                let line_count = function_source.lines().count();
                if line_count > max_length {
                    diagnostics.push(Diagnostic::new_with_severity(
                        DiagnosticKind::Linter,
                        severity,
                        format!(
                            "Function \"{}\" has {} lines (exceeds maximum of {})",
                            name, line_count, max_length
                        ),
                        span.clone(),
                    ));
                }
            }
        }
        diagnostics
    }
}
