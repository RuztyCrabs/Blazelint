use crate::ast::Stmt;
use crate::errors::Diagnostic;
use crate::linter::Rule;
use crate::semantic;

/// Linter rule that checks for functions with non-void return types that might not return a value on all code paths.
#[derive(Debug, Default)]
pub struct MissingReturn;

impl MissingReturn {
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

impl Rule for MissingReturn {
    fn name(&self) -> &'static str {
        "missing_return"
    }

    fn description(&self) -> &'static str {
        "Detects functions with non-void return types that might not return a value on all code paths."
    }

    fn validate(&self, _statement: &Stmt, _source: &str) -> Vec<Diagnostic> {
        // This rule is validated at the AST level, so this method is empty.
        vec![]
    }

    fn validate_ast(&self, ast: &[Stmt], _source: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if semantic::analyze(ast).is_err() {
            // Semantic errors exist, don't run this rule
            return diagnostics;
        }

        for stmt in ast {
            if let Stmt::Function {
                name,
                return_type,
                body,
                ..
            } = stmt
            {
                if return_type.is_some() && !self.check_returns_in_block(body) {
                    diagnostics.push(Diagnostic::new(
                        crate::errors::DiagnosticKind::Linter,
                        format!(
                            "Function '{}' might not return a value on all code paths.",
                            name
                        ),
                        stmt.span().clone(),
                    ));
                }
            }
        }
        diagnostics
    }
}
