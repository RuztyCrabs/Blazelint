use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
};

/// A linting rule to enforce that constant variable names are in SCREAMING_SNAKE_CASE.
pub struct ConstantCaseRule;

impl LintRule for ConstantCaseRule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "constant_case"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Constant variable names should be in SCREAMING_SNAKE_CASE."
    }

    /// Returns the severity of the rule.
    fn severity(&self) -> Severity {
        Severity::Info
    }

    /// Checks the given abstract syntax tree (AST) for violations of the rule.
    fn check(&self, ast: &[Stmt], _file_path: &str, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for stmt in ast {
            check_and_enforce_constant_case(stmt, &mut diagnostics, source, self.severity());
        }
        diagnostics
    }
}

/// Recursively checks for constant declarations and enforces SCREAMING_SNAKE_CASE.
#[allow(unused_variables)]
fn check_and_enforce_constant_case(
    stmt: &Stmt,
    diagnostics: &mut Vec<Diagnostic>,
    source: &str, // Reverted to source: &str
    severity: Severity,
) {
    if let Stmt::ConstDecl {
        name, name_span, ..
    } = stmt
    {
        if !is_screaming_snake_case(name) {
            diagnostics.push(Diagnostic::new_with_severity(
                DiagnosticKind::Linter,
                severity,
                format!(
                    "Constant variable \"{}\" is not in SCREAMING_SNAKE_CASE.",
                    name
                ),
                name_span.clone(),
            ));
        }
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
