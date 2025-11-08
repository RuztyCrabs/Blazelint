use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
};

const MAX_LINE_LENGTH: usize = 120;

/// A linting rule to enforce that lines do not exceed a maximum length.
pub struct LineLengthRule;

impl LintRule for LineLengthRule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "line_length"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Lines should not exceed 120 characters."
    }

    /// Returns the severity of the rule.
    fn severity(&self) -> Severity {
        Severity::Warning
    }

    /// Checks the given source code for lines that exceed the maximum length.
    fn check(&self, _ast: &[Stmt], _file_path: &str, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let mut offset = 0;
        for line in source.lines() {
            if line.len() > MAX_LINE_LENGTH {
                let pos = crate::utils::get_line_and_column(offset, source);
                diagnostics.push(Diagnostic::new_with_severity(
                    DiagnosticKind::Linter,
                    self.severity(),
                    self.description().to_string(),
                    pos.line..pos.column, // Span for the diagnostic
                ));
            }
            offset += line.len() + 1;
        }
        diagnostics
    }
}
