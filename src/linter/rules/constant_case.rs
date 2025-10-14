use crate::ast::Stmt;
use crate::errors::{Diagnostic, DiagnosticKind};
use crate::linter::Rule;

/// A linting rule to enforce that constant variable names are in SCREAMING_SNAKE_CASE.
pub struct ConstantCase;

impl Rule for ConstantCase {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "constant-case"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Constant variable names should be in SCREAMING_SNAKE_CASE."
    }

    /// Validates a given statement to ensure that constant variable names are in SCREAMING_SNAKE_CASE.
    ///
    /// # Arguments
    ///
    /// * `statement` - The statement to validate.
    ///
    /// # Returns
    ///
    /// A vector of diagnostics if the constant variable name is not in SCREAMING_SNAKE_CASE.
    fn validate(&self, statement: &Stmt, _source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Stmt::ConstDecl {
            name, name_span, ..
        } = statement
        {
            if !is_screaming_snake_case(name) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticKind::Linter,
                    "Constant variable names should be in SCREAMING_SNAKE_CASE.".to_string(),
                    name_span.clone(),
                ));
            }
        }

        diagnostics
    }
}

/// Checks if a given string is in SCREAMING_SNAKE_CASE.
///
/// # Arguments
///
/// * `name` - The string to check.
///
/// # Returns
///
/// `true` if the string is in SCREAMING_SNAKE_CASE, `false` otherwise.
fn is_screaming_snake_case(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii_uppercase() || c == '_')
}
