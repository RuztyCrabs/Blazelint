use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
};

/// A rule that enforces variable names to be in camelCase.
///
/// This rule checks for variable declarations and reports a diagnostic
/// if the variable name is not in camelCase.
pub struct CamelCaseRule;

impl LintRule for CamelCaseRule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "camel_case"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Variables should be in camelCase."
    }

    /// Returns the severity of the rule.
    fn severity(&self) -> Severity {
        Severity::Info
    }

    /// Checks the given abstract syntax tree (AST) for violations of the rule.
    fn check(&self, ast: &[Stmt], _file_path: &str, source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for stmt in ast {
            check_and_enforce_camel_case(stmt, &mut diagnostics, source, self.severity());
        }
        diagnostics
    }
}

/// Recursively checks for variable declarations and enforces camelCase.
#[allow(clippy::only_used_in_recursion)]
fn check_and_enforce_camel_case(
    stmt: &Stmt,
    diagnostics: &mut Vec<Diagnostic>,
    source: &str, // Reverted to source: &str
    severity: Severity,
) {
    match stmt {
        Stmt::VarDecl {
            name, name_span, ..
        } => {
            if !is_camel_case(name) {
                diagnostics.push(Diagnostic::new_with_severity(
                    DiagnosticKind::Linter,
                    severity,
                    format!("Variable \"{}\" is not in camelCase.", name),
                    name_span.clone(),
                ));
            }
        }
        Stmt::Function { body, .. } => {
            for s in body {
                check_and_enforce_camel_case(s, diagnostics, source, severity);
            }
        }
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => {
            for s in then_branch {
                check_and_enforce_camel_case(s, diagnostics, source, severity);
            }
            if let Some(else_branch) = else_branch {
                for s in else_branch {
                    check_and_enforce_camel_case(s, diagnostics, source, severity);
                }
            }
        }
        Stmt::While { body, .. } => {
            for s in body {
                check_and_enforce_camel_case(s, diagnostics, source, severity);
            }
        }
        Stmt::Foreach { body, .. } => {
            for s in body {
                check_and_enforce_camel_case(s, diagnostics, source, severity);
            }
        }
        _ => {}
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
