use crate::{
    ast::{MatchPattern, Stmt, TypeDescriptor},
    config::Config,
    errors::{Diagnostic, DiagnosticKind},
    linter::registry::LintRule,
};

/// Linter rule that checks for functions with non-void return types that might not return a value on all code paths.
pub struct MissingReturnRule;

impl Default for MissingReturnRule {
    fn default() -> Self {
        Self::new()
    }
}

impl MissingReturnRule {
    /// Creates a new `MissingReturn` rule.
    pub fn new() -> Self {
        Self
    }

    /// Returns true when a return type permits a function to fall off the end
    /// without an explicit return (i.e. it can be nil): `T?`, `()`, or a union
    /// containing nil. Such functions implicitly return `()`.
    fn allows_implicit_nil(ty: &TypeDescriptor) -> bool {
        match ty {
            TypeDescriptor::Optional(_) => true,
            TypeDescriptor::Basic(name) => name == "()" || name == "nil",
            TypeDescriptor::Union(members) => members.iter().any(Self::allows_implicit_nil),
            _ => false,
        }
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
            Stmt::Return { .. } | Stmt::Panic { .. } | Stmt::Fail { .. } => true,
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
            Stmt::Block { body, .. } => self.check_returns_in_block(body),
            Stmt::Match { arms, .. } => {
                // A match returns on all paths only if it is exhaustive (has an
                // unguarded catch-all arm) and every arm body returns.
                let has_catch_all = arms.iter().any(|arm| {
                    arm.guard.is_none()
                        && arm
                            .patterns
                            .iter()
                            .any(|p| matches!(p, MatchPattern::Wildcard | MatchPattern::Binding(_)))
                });
                has_catch_all
                    && arms
                        .iter()
                        .all(|arm| self.check_returns_in_block(&arm.body))
            }
            _ => false,
        }
    }
}

impl LintRule for MissingReturnRule {
    fn name(&self) -> &'static str {
        "missing-return"
    }

    fn description(&self) -> &'static str {
        "Detects functions with non-void return types that might not return a value on all code paths."
    }

    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        _source: &str,
        config: &Config,
        line_tracker: &crate::utils::LineTracker,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let severity = self.severity(config);
        for stmt in ast {
            if let Stmt::Function {
                name,
                return_type,
                body,
                span,
                ..
            } = stmt
            {
                let requires_value = return_type
                    .as_ref()
                    .is_some_and(|ty| !Self::allows_implicit_nil(ty));
                if requires_value && !self.check_returns_in_block(body) {
                    diagnostics.push(Diagnostic::new_tracked(
                        DiagnosticKind::Linter,
                        severity,
                        format!(
                            "Function '{}' might not return a value on all code paths.",
                            name
                        ),
                        span.clone(),
                        line_tracker,
                    ));
                }
            }
        }
        diagnostics
    }
}
