//! Rule to enforce camelCase for variable names.

use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind},
    linter::Rule,
};

/// A rule that enforces variable names to be in camelCase.
///
/// This rule checks for variable declarations and reports a diagnostic
/// if the variable name is not in camelCase.
pub struct CamelCase;

impl Rule for CamelCase {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "camel_case"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Variables should be in camelCase."
    }

    /// Validates the given statement against the rule.
    ///
    /// This function checks if the statement is a variable declaration and
    /// if the variable name is in camelCase.
    ///
    /// # Arguments
    ///
    /// * `statement` - The statement to validate
    ///
    /// # Returns
    ///
    /// A vector of diagnostics found in the statement
    fn validate(&self, statement: &Stmt) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        if let Stmt::VarDecl {
            name, name_span, ..
        } = statement
        {
            if !is_camel_case(name) {
                diagnostics.push(Diagnostic::new(
                    DiagnosticKind::Linter,
                    format!("Variable \"{}\" is not in camelCase.", name),
                    name_span.clone(),
                ));
            }
        }

        diagnostics
    }
}

/// Checks if a string is in camelCase.
///
/// A string is considered to be in camelCase if it starts with a lowercase
/// ASCII letter, and all other characters are alphanumeric and there are no
/// unerscores.
///
/// # Arguments
///
/// * `s` - The string to check.
///
/// # Returns
///
/// `true` if the string is in camelCase, `false` otherwise.
fn is_camel_case(s: &str) -> bool {
    let mut chars = s.chars();
    if let Some(first) = chars.next() {
        if !first.is_ascii_lowercase() {
            return false;
        }
    }
    s.chars().all(|c| c.is_ascii_alphanumeric()) && !s.contains('_')
}
