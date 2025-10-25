use crate::ast::{Expr, Stmt};
use crate::errors::{Diagnostic, DiagnosticKind, Span};
use crate::linter::Rule;
use std::collections::HashMap;

#[derive(Default)]
pub struct UnusedVariables;

impl Rule for UnusedVariables {
    fn name(&self) -> &'static str {
        "unused_variables"
    }

    fn description(&self) -> &'static str {
        "Detects unused variables in the code."
    }

    fn validate(&self, statement: &Stmt, _source: &str) -> Vec<Diagnostic> {
        let mut visitor = Visitor::new();
        visitor.visit_stmt(statement);
        visitor.errors
    }
}

struct Visitor {
    scopes: Vec<HashMap<String, (Span, bool)>>,
    errors: Vec<Diagnostic>,
}

impl Visitor {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            errors: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for (name, (span, used)) in scope {
                if !used && !name.starts_with('_') {
                    self.errors.push(Diagnostic {
                        kind: DiagnosticKind::Linter,
                        message: format!("Variable '{}' is never used", name),
                        span,
                        notes: vec![],
                    });
                }
            }
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Function { body, params, .. } => {
                self.enter_scope();
                for (name, _) in params {
                    if let Some(scope) = self.scopes.last_mut() {
                        // We don't have a span for the parameter name, so we use the function span
                        scope.insert(name.clone(), (stmt.span().clone(), false));
                    }
                }
                for statement in body {
                    self.visit_stmt(statement);
                }
                self.exit_scope();
            }
            Stmt::VarDecl { name, name_span, initializer, .. } => {
                if let Some(scope) = self.scopes.last_mut() {
                    scope.insert(name.clone(), (name_span.clone(), false));
                }
                if let Some(expr) = initializer {
                    self.visit_expr(expr);
                }
            }
            Stmt::Expression { expression, .. } => self.visit_expr(expression),
            Stmt::Return { value, .. } => {
                if let Some(expr) = value {
                    self.visit_expr(expr);
                }
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                self.visit_expr(condition);
                for statement in then_branch {
                    self.visit_stmt(statement);
                }
                if let Some(else_body) = else_branch {
                    for statement in else_body {
                        self.visit_stmt(statement);
                    }
                }
            }
            _ => {},
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable { name, .. } => {
                for scope in self.scopes.iter_mut().rev() {
                    if let Some(decl) = scope.get_mut(name) {
                        decl.1 = true;
                        break;
                    }
                }
            }
            Expr::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::Unary { operand, .. } => {
                self.visit_expr(operand);
            }
            Expr::Call { callee, arguments, .. } => {
                self.visit_expr(callee);
                for arg in arguments {
                    self.visit_expr(arg);
                }
            }
            _ => {},
        }
    }
}
