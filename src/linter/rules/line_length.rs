use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind},
    linter::Rule,
};

const MAX_LINE_LENGTH: usize = 120;

/// A linting rule to enforce that lines do not exceed a maximum length.
#[derive(Debug, Clone)]
pub struct LineLength;

impl Rule for LineLength {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "line_length"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Lines should not exceed 120 characters."
    }

    /// Validates a given statement to ensure that it does not exceed the maximum line length.
    ///
    /// # Arguments
    ///
    /// * `statement` - The statement to validate.
    /// * `source` - The source code of the file being linted.
    ///
    /// # Returns
    ///
    /// A vector of diagnostics if the statement exceeds the maximum line length.
    fn validate(&self, statement: &Stmt, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let span = statement.span();

        let statement_source = &source[span.start..span.end];

        for line in statement_source.lines() {
            if line.len() > MAX_LINE_LENGTH {
                diagnostics.push(Diagnostic::new(
                    DiagnosticKind::Linter,
                    self.description().to_string(),
                    span.clone(),
                ));
                // We only want to report the error once per statement
                break;
            }
        }

        diagnostics
    }
}
