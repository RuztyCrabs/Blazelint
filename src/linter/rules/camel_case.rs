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
    /// This function is a no-op because this rule uses `validate_ast`.
    fn validate(&self, _statement: &Stmt, _source: &str) -> Vec<Diagnostic> {
        Vec::new()
    }

    fn validate_ast(&self, ast: &[Stmt], _source: &str) -> Vec<Diagnostic> {
        let mut visitor = CamelCaseVisitor::new();
        visitor.visit_stmts(ast);
        visitor.diagnostics
    }
}

struct CamelCaseVisitor {
    diagnostics: Vec<Diagnostic>,
}

impl CamelCaseVisitor {
    fn new() -> Self {
        Self {
            diagnostics: Vec::new(),
        }
    }

    fn visit_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name, name_span, ..
            } => {
                if !is_camel_case(name) {
                    self.diagnostics.push(Diagnostic::new(
                        DiagnosticKind::Linter,
                        format!("Variable \"{}\" is not in camelCase.", name),
                        name_span.clone(),
                    ));
                }
            }
            Stmt::Function { body, .. } => self.visit_stmts(body),
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                self.visit_stmts(then_branch);
                if let Some(else_branch) = else_branch {
                    self.visit_stmts(else_branch);
                }
            }
            Stmt::While { body, .. } => self.visit_stmts(body),
            Stmt::Foreach { body, .. } => self.visit_stmts(body),
            _ => {}
        }
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
