use crate::{
    ast::Stmt,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
};

/// Linter rule that checks for functions with non-void return types that might not return a value on all code paths.
pub struct MissingReturnRule;

impl MissingReturnRule {
    /// Creates a new `MissingReturn` rule.
    pub fn new() -> Self {
        Self
    }

    /// Recursively checks if a block of statements guarantees a return.
    fn check_returns_in_block(&self, stmts: &[Stmt]) -> bool {
        for statement in stmts {
            if self.statement_returns(statement) {
                return true;
            }
        }
        false
    }

    /// Checks if a single statement guarantees a return.
    fn statement_returns(&self, statement: &Stmt) -> bool {
        match statement {
            Stmt::Return { .. } => true,
            Stmt::If {
                then_branch,
                else_branch,
                ..
            } => {
                if !self.check_returns_in_block(then_branch) {
                    return false;
                }
                if let Some(else_branch) = else_branch {
                    if !self.check_returns_in_block(else_branch) {
                        return false;
                    }
                } else {
                    return false;
                }
                true
            }
            _ => false,
        }
    }
}

impl LintRule for MissingReturnRule {
    fn name(&self) -> &'static str {
        "missing_return"
    }

    fn description(&self) -> &'static str {
        "Detects functions with non-void return types that might not return a value on all code paths."
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn check(&self, ast: &[Stmt], _file_path: &str, _source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for stmt in ast {
            if let Stmt::Function {
                name,
                return_type,
                body,
                span,
                ..
            } = stmt
            {
                if return_type.is_some() && !self.check_returns_in_block(body) {
                    diagnostics.push(Diagnostic::new_with_severity(
                        DiagnosticKind::Linter,
                        self.severity(),
                        format!(
                            "Function '{}' might not return a value on all code paths.",
                            name
                        ),
                        span.clone(),
                    ));
                }
            }
        }
        diagnostics
    }
}
