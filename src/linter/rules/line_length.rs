use crate::{
    ast::Stmt,
    config::Config,
    errors::{Diagnostic, DiagnosticKind},
    linter::registry::LintRule,
};

/// A linting rule to enforce that lines do not exceed a maximum length.
pub struct LineLengthRule;

impl LintRule for LineLengthRule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "line-length"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Lines should not exceed the configured maximum length."
    }

    /// Checks the given source code for lines that exceed the maximum length.
    fn check(
        &self,
        _ast: &[Stmt],
        _file_path: &str,
        source: &str,
        config: &Config,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let max_line_length = config.settings.max_line_length as usize;
        let severity = self.severity(config);
        let mut offset = 0;
        for line in source.lines() {
            if line.len() > max_line_length {
                let span = offset..offset + line.len();
                diagnostics.push(Diagnostic::new_with_severity(
                    DiagnosticKind::Linter,
                    severity,
                    format!("Line exceeds {} characters.", max_line_length),
                    span, // Span for the diagnostic
                ));
            }
            offset += line.len() + 1;
        }
        diagnostics
    }
}
