pub mod rules;

use crate::{ast::Stmt, errors::Diagnostic};

/// A blueprint for creating new linting rules.
#[allow(dead_code)]
pub trait Rule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str;

    /// Returns a description of the rule.
    fn description(&self) -> &'static str;

    /// Validates a given statement.
    fn validate(&self, statement: &Stmt, source: &str) -> Vec<Diagnostic>;

    /// Validates the entire AST.
    fn validate_ast(&self, ast: &[Stmt], source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for stmt in ast {
            diagnostics.extend(self.validate(stmt, source));
        }
        diagnostics
    }
}
