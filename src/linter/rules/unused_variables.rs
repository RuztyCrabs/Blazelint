//! Rule to detect unused variables.

use crate::{
    ast::{Expr, Stmt},
    errors::{Diagnostic, DiagnosticKind, Position, Severity},
    linter::registry::LintRule,
};
use std::collections::HashMap;

/// A rule that detects unused variables.
///
/// This rule traverses the AST, tracks variable declarations and usages,
/// and emits a linter diagnostic for each variable that is declared but never used.
pub struct UnusedVariablesRule;

impl LintRule for UnusedVariablesRule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "unused_variables"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Detects unused variables."
    }

    /// Returns the severity of the rule.
    fn severity(&self) -> Severity {
        Severity::Error
    }

    /// Validates the entire AST for unused variables.
    fn check(&self, ast: &[Stmt], _file_path: &str, source: &str) -> Vec<Diagnostic> {
        let mut visitor = UnusedVariableVisitor::new(source, self.severity());
        visitor.visit_stmts(ast);
        visitor.exit_scope(); // Exit the global scope
        visitor.diagnostics
    }
}

/// Information about a variable's declaration and usage status.
#[derive(Debug, Clone)]
struct VariableInfo {
    /// The position in the source code where the variable was declared.
    declaration_pos: Position,
    /// Whether the variable was used.
    used: bool,
}

/// Visitor that traverses the AST to track variable usage and collect diagnostics for unused variables.
pub struct UnusedVariableVisitor<'a> {
    /// Stack of variable scopes (for block scoping).
    scopes: Vec<HashMap<String, VariableInfo>>,
    /// Collected diagnostics for unused variables.
    diagnostics: Vec<Diagnostic>,
    source: &'a str,
    severity: Severity,
}

impl<'a> UnusedVariableVisitor<'a> {
    /// Creates a new UnusedVariableVisitor with an initial (global) scope.
    pub fn new(source: &'a str, severity: Severity) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
            source,
            severity,
        }
    }

    /// Enters a new variable scope (e.g., for a function or block).
    fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    /// Exits the current variable scope, emitting diagnostics for any unused variables.
    fn exit_scope(&mut self) {
        if let Some(scope) = self.scopes.pop() {
            for (name, info) in scope {
                if !info.used && !name.starts_with('_') {
                    self.diagnostics.push(Diagnostic::new_with_severity(
                        DiagnosticKind::Linter,
                        self.severity,
                        format!("Variable {} is never used", name),
                        info.declaration_pos.line..info.declaration_pos.column, // Convert Position to Span
                    ));
                }
            }
        }
    }

    /// Declares a new variable in the current scope.
    fn declare_variable(&mut self, name: String, pos: Position) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(
                name,
                VariableInfo {
                    declaration_pos: pos,
                    used: false,
                },
            );
        }
    }

    /// Marks a variable as used, searching from innermost to outermost scope.
    fn use_variable(&mut self, name: &str) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.used = true;
                return;
            }
        }
    }

    /// Visits a list of statements, tracking variable usage.
    pub fn visit_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.visit_stmt(stmt);
        }
    }

    /// Visits a single statement, handling variable declarations, function scopes, and control flow.
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
                let pos = crate::utils::get_line_and_column(name_span.start, self.source);
                self.declare_variable(name.clone(), pos);
            }
            Stmt::Function { body, params, .. } => {
                self.enter_scope();
                for (name, _) in params {
                    // FIXME: We don't have a span for the parameter name
                    self.declare_variable(name.clone(), Position::new(0, 0));
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
                span,
                ..
            } => {
                self.visit_expr(iterable);
                self.enter_scope();
                let pos = crate::utils::get_line_and_column(span.start, self.source);
                self.declare_variable(variable.clone(), pos);
                self.visit_stmts(body);
                self.exit_scope();
            }
            Stmt::Expression { expression, .. } => self.visit_expr(expression),
            Stmt::Return {
                value: Some(val), ..
            } => self.visit_expr(val),
            _ => {}
        }
    }

    /// Visits an expression, tracking variable usage recursively.
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
            _ => {}
        }
    }
}
