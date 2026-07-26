//! Rule to detect unused variables.

use crate::{
    ast::{Expr, QueryClause, Stmt},
    config::Config,
    errors::{Diagnostic, DiagnosticKind, Severity},
    linter::registry::LintRule,
};
use std::collections::HashMap;
use std::ops::Range;

/// A rule that detects unused variables.
///
/// This rule traverses the AST, tracks variable declarations and usages,
/// and emits a linter diagnostic for each variable that is declared but never used.
pub struct UnusedVariablesRule;

impl LintRule for UnusedVariablesRule {
    /// Returns the name of the rule.
    fn name(&self) -> &'static str {
        "unused-variables"
    }

    /// Returns a description of the rule.
    fn description(&self) -> &'static str {
        "Detects unused variables."
    }

    /// Validates the entire AST for unused variables.
    fn check(
        &self,
        ast: &[Stmt],
        _file_path: &str,
        source: &str,
        config: &Config,
        line_tracker: &crate::utils::LineTracker,
    ) -> Vec<Diagnostic> {
        let severity = self.severity(config);
        let mut visitor = UnusedVariableVisitor::new(source, severity, line_tracker);
        visitor.visit_stmts(ast);
        visitor.exit_scope(); // Exit the global scope
        visitor.diagnostics
    }
}

/// Information about a variable's declaration and usage status.
#[derive(Debug, Clone)]
struct VariableInfo {
    /// The span in the source code where the variable was declared.
    declaration_span: Range<usize>,
    /// Whether the variable was used.
    used: bool,
}

/// Visitor that traverses the AST to track variable usage and collect diagnostics for unused variables.
struct UnusedVariableVisitor<'a> {
    /// Stack of variable scopes. Each scope maps variable names to their information.
    scopes: Vec<HashMap<String, VariableInfo>>,
    /// The collected diagnostics found during the walk.
    diagnostics: Vec<Diagnostic>,
    _source: &'a str,
    severity: Severity,
    line_tracker: &'a crate::utils::LineTracker,
}

impl<'a> UnusedVariableVisitor<'a> {
    /// Creates a new UnusedVariableVisitor with an initial (global) scope.
    pub fn new(
        source: &'a str,
        severity: Severity,
        line_tracker: &'a crate::utils::LineTracker,
    ) -> Self {
        Self {
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
            _source: source,
            severity,
            line_tracker,
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
                    self.diagnostics.push(Diagnostic::new_tracked(
                        DiagnosticKind::Linter,
                        self.severity,
                        format!("Variable {} is never used", name),
                        info.declaration_span.clone(),
                        self.line_tracker,
                    ));
                }
            }
        }
    }

    /// Declares a new variable in the current scope.
    fn declare_variable(&mut self, name: String, span: Range<usize>) {
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
                self.declare_variable(name.clone(), name_span.clone());
            }
            Stmt::Function {
                body,
                params,
                name_span,
                ..
            } => {
                self.enter_scope();
                for (name, _) in params {
                    // FIXME: We don't have a span for the parameter name
                    self.declare_variable(name.clone(), name_span.clone());
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
                self.declare_variable(variable.clone(), span.clone());
                self.visit_stmts(body);
                self.exit_scope();
            }
            Stmt::Expression { expression, .. } => self.visit_expr(expression),
            Stmt::Return {
                value: Some(val), ..
            } => self.visit_expr(val),
            Stmt::Panic { value, .. } | Stmt::Fail { value, .. } => self.visit_expr(value),
            Stmt::ConstDecl { initializer, .. } => self.visit_expr(initializer),
            Stmt::DestructureDecl {
                names,
                name_spans,
                initializer,
                ..
            } => {
                self.visit_expr(initializer);
                for (name, span) in names.iter().zip(name_spans.iter()) {
                    self.declare_variable(name.clone(), span.clone());
                }
            }
            Stmt::Block { body, .. }
            | Stmt::Lock { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Retry { body, .. }
            | Stmt::Worker { body, .. } => {
                self.enter_scope();
                self.visit_stmts(body);
                self.exit_scope();
            }
            Stmt::Match { subject, arms, .. } => {
                self.visit_expr(subject);
                for arm in arms {
                    // Pattern-bound names (captures/rests) are intentionally not
                    // declared for unused-tracking: match catch-alls frequently
                    // ignore their binding, so flagging them would be noisy.
                    self.enter_scope();
                    if let Some(guard) = &arm.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_stmts(&arm.body);
                    self.exit_scope();
                }
            }
            Stmt::DoOnFail {
                body, on_fail_body, ..
            } => {
                self.enter_scope();
                self.visit_stmts(body);
                self.exit_scope();
                self.enter_scope();
                self.visit_stmts(on_fail_body);
                self.exit_scope();
            }
            Stmt::ClassDef { members, .. } | Stmt::ServiceDecl { members, .. } => {
                // Visit method bodies and field initializers so their references
                // to module-level variables are tracked. Fields themselves are not
                // declared for unused-tracking (accessed via `self`, not by name).
                for member in members {
                    match member {
                        Stmt::Function { .. } => self.visit_stmt(member),
                        Stmt::VarDecl {
                            initializer: Some(init),
                            ..
                        } => self.visit_expr(init),
                        _ => {}
                    }
                }
            }
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
            Expr::MemberAccess { object, member, .. } => {
                self.visit_expr(object);
                self.visit_expr(member);
            }
            Expr::FieldAccess { object, .. } => self.visit_expr(object),
            Expr::MemberAssign { target, value, .. } => {
                self.visit_expr(target);
                self.visit_expr(value);
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
                for (_key, value) in entries {
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
            Expr::New { arguments, .. } => {
                for arg in arguments {
                    self.visit_expr(arg);
                }
            }
            Expr::Check { expr, .. } | Expr::TypeOf { expr, .. } | Expr::TypeTest { expr, .. } => {
                self.visit_expr(expr)
            }
            Expr::StringTemplate { interpolations, .. } => {
                for interp in interpolations {
                    self.visit_expr(interp);
                }
            }
            Expr::RemoteCall {
                object, arguments, ..
            } => {
                self.visit_expr(object);
                for arg in arguments {
                    self.visit_expr(arg);
                }
            }
            Expr::Let { bindings, body, .. } => {
                self.enter_scope();
                for binding in bindings {
                    self.visit_expr(&binding.value);
                    self.declare_variable(binding.name.clone(), 0..0);
                }
                self.visit_expr(body);
                self.exit_scope();
            }
            Expr::AnonFunction { body, .. } => {
                self.enter_scope();
                self.visit_stmts(body);
                self.exit_scope();
            }
            Expr::Arrow { body, .. } => {
                self.enter_scope();
                self.visit_expr(body);
                self.exit_scope();
            }
            Expr::Query { clauses, .. } => {
                // Visit every sub-expression so outer variables referenced in the
                // query are marked used. Query-clause bindings are not declared for
                // unused-tracking (same rationale as match bindings).
                self.enter_scope();
                for clause in clauses {
                    match clause {
                        QueryClause::From { source, .. } => self.visit_expr(source),
                        QueryClause::Where(expr)
                        | QueryClause::Limit(expr)
                        | QueryClause::Select(expr) => self.visit_expr(expr),
                        QueryClause::Let(bindings) => {
                            for binding in bindings {
                                self.visit_expr(&binding.value);
                            }
                        }
                        QueryClause::Join {
                            source,
                            on_left,
                            on_right,
                            ..
                        } => {
                            self.visit_expr(source);
                            self.visit_expr(on_left);
                            self.visit_expr(on_right);
                        }
                        QueryClause::OrderBy(keys) => {
                            for key in keys {
                                self.visit_expr(key);
                            }
                        }
                        QueryClause::Do(body) => self.visit_stmts(body),
                        QueryClause::Other => {}
                    }
                }
                self.exit_scope();
            }
            Expr::TableConstructor { rows, .. } => {
                for row in rows {
                    self.visit_expr(row);
                }
            }
            Expr::Start { call, .. } => self.visit_expr(call),
            Expr::ObjectConstructor { members, .. } => {
                for member in members {
                    if let Stmt::Function { .. } = member {
                        self.visit_stmt(member);
                    }
                }
            }
            Expr::Literal { .. } => {}
        }
    }
}
