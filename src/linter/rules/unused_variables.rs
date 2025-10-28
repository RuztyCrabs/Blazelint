//! Rule to detect unused variables.

use crate::{
    ast::{Expr, Stmt},
    errors::{Diagnostic, DiagnosticKind, Span},
    linter::Rule,
};
use std::collections::HashMap;

/// A rule that detects unused variables.
pub struct UnusedVariables;

impl Rule for UnusedVariables {
    fn name(&self) -> &'static str {
        "unused-variables"
    }

    fn description(&self) -> &'static str {
        "Detects unused variables."
    }

    fn validate(&self, _statement: &Stmt, _source: &str) -> Vec<Diagnostic> {
        // This rule uses validate_ast, so this is a no-op.
        Vec::new()
    }

    fn validate_ast(&self, ast: &[Stmt], _source: &str) -> Vec<Diagnostic> {
        let mut visitor = UnusedVariableVisitor::new();
        visitor.visit_stmts(ast);
        visitor.exit_scope(); // Exit the global scope
        visitor.diagnostics
    }
}

#[derive(Debug, Clone)]
struct VariableInfo {
    declaration_span: Span,
    used: bool,
}

pub struct UnusedVariableVisitor {
    scopes: Vec<HashMap<String, VariableInfo>>,
    diagnostics: Vec<Diagnostic>,
}

impl UnusedVariableVisitor {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
        }
    }

    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for (name, info) in scope {
                if !info.used && !name.starts_with('_') {
                    self.diagnostics.push(Diagnostic::new(
                        DiagnosticKind::Linter,
                        format!("linter error: Variable {} is never used", name),
                        info.declaration_span,
                    ));
                }
            }
        }
    }

    fn declare_variable(&mut self, name: String, span: Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name,
                VariableInfo {
                    declaration_span: span,
                    used: false,
                },
            );
        }
    }

    fn use_variable(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.used = true;
                return;
            }
        }
    }

    pub fn visit_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::VarDecl {
                name,
                name_span,
                initializer,
                ..
            } => {
                if let Some(init) = initializer {
                    self.visit_expr(init);
                }
                self.declare_variable(name.clone(), name_span.clone());
            }
            Stmt::Function { body, params, .. } => {
                self.enter_scope();
                for (name, _) in params {
                    // No span for param name, so use a dummy span or skip
                    self.declare_variable(name.clone(), Span { start: 0, end: 0 });
                }
                self.visit_stmts(body);
                self.exit_scope();
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                self.visit_expr(condition);
                self.enter_scope();
                self.visit_stmts(then_branch);
                self.exit_scope();
                if let Some(else_branch) = else_branch {
                    self.enter_scope();
                    self.visit_stmts(else_branch);
                    self.exit_scope();
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.visit_expr(condition);
                self.enter_scope();
                self.visit_stmts(body);
                self.exit_scope();
            }
            Stmt::Foreach {
                variable,
                iterable,
                body,
                ..
            } => {
                self.visit_expr(iterable);
                self.enter_scope();
                // We need a span for the variable, but it's not in the AST yet.
                // For now, let's use the iterable's span.
                self.declare_variable(variable.clone(), iterable.span().clone());
                self.visit_stmts(body);
                self.exit_scope();
            }
            Stmt::Expression { expression, .. } => self.visit_expr(expression),
            Stmt::Return {
                value: Some(val), ..
            } => self.visit_expr(val),
            Stmt::Panic { value, .. } => self.visit_expr(value),
            // Other statements don't introduce scopes or variables
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Variable { name, .. } => self.use_variable(name),
            Expr::Binary { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::Unary { operand, .. } => self.visit_expr(operand),
            Expr::Grouping { expression, .. } => self.visit_expr(expression),
            Expr::Call {
                callee, arguments, ..
            } => {
                self.visit_expr(callee);
                for arg in arguments {
                    self.visit_expr(arg);
                }
            }
            Expr::Assign { name, value, .. } => {
                self.use_variable(name);
                self.visit_expr(value);
            }
            Expr::MemberAccess { object, member, .. } => {
                self.visit_expr(object);
                self.visit_expr(member);
            }
            Expr::MethodCall {
                object, arguments, ..
            } => {
                self.visit_expr(object);
                for arg in arguments {
                    self.visit_expr(arg);
                }
            }
            Expr::ArrayLiteral { elements, .. } => {
                for element in elements {
                    self.visit_expr(element);
                }
            }
            Expr::MapLiteral { entries, .. } => {
                for (_, value) in entries {
                    self.visit_expr(value);
                }
            }
            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                self.visit_expr(condition);
                self.visit_expr(true_expr);
                self.visit_expr(false_expr);
            }
            Expr::Elvis { expr, default, .. } => {
                self.visit_expr(expr);
                self.visit_expr(default);
            }
            Expr::Range { start, end, .. } => {
                self.visit_expr(start);
                self.visit_expr(end);
            }
            Expr::Cast { expr, .. } => self.visit_expr(expr),
            Expr::Literal { .. } => {
                // Literals don't contain variables
            }
        }
    }
}
