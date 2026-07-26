//! Shared AST traversal for lint rules.
//!
//! Rules that need to inspect every node should use these walkers rather than
//! re-implementing recursion. Each rule then only describes *what* it looks for,
//! and gains coverage of new syntax automatically when the walkers are extended.
//!
//! The walkers are exhaustive over `Stmt` and `Expr`, so adding an AST variant
//! is a compile error here — which is the point: it forces a decision about how
//! the new node is traversed instead of silently skipping it.

use crate::ast::{Expr, QueryClause, Stmt};

/// Calls `f` on every statement in `stmts`, including nested ones.
pub fn walk_stmts<F: FnMut(&Stmt)>(stmts: &[Stmt], f: &mut F) {
    for stmt in stmts {
        walk_stmt(stmt, f);
    }
}

/// Calls `f` on `stmt` and every statement nested within it — including bodies
/// carried by expressions, such as an anonymous function, an object
/// constructor, or a query `do` clause.
pub fn walk_stmt<F: FnMut(&Stmt)>(stmt: &Stmt, f: &mut F) {
    f(stmt);
    for_each_child_stmt(stmt, &mut |s| walk_stmt(s, f));
    // Statement bodies held inside this statement's own expressions.
    for_each_direct_expr(stmt, &mut |e| {
        walk_expr(e, &mut |inner| {
            for_each_expr_body(inner, &mut |s| walk_stmt(s, f));
        })
    });
}

/// Applies `g` to each statement in a body carried by an expression.
fn for_each_expr_body<G: FnMut(&Stmt)>(expr: &Expr, g: &mut G) {
    match expr {
        Expr::AnonFunction { body, .. } => g_all(body, g),
        Expr::ObjectConstructor { members, .. } => g_all(members, g),
        Expr::Query { clauses, .. } => {
            for clause in clauses {
                if let QueryClause::Do(body) = clause {
                    g_all(body, g);
                }
            }
        }
        _ => {}
    }
}

/// Calls `f` on every expression reachable from `stmts`, including those nested
/// inside other expressions and inside nested statements.
pub fn walk_exprs<F: FnMut(&Expr)>(stmts: &[Stmt], f: &mut F) {
    walk_stmts(stmts, &mut |stmt| {
        for_each_direct_expr(stmt, &mut |e| walk_expr(e, f));
    });
}

/// Calls `f` on `expr` and every sub-expression.
pub fn walk_expr<F: FnMut(&Expr)>(expr: &Expr, f: &mut F) {
    f(expr);
    for_each_child_expr(expr, &mut |e| walk_expr(e, f));
}

/// Applies `g` to each statement directly nested in `stmt` (one level).
fn for_each_child_stmt<G: FnMut(&Stmt)>(stmt: &Stmt, g: &mut G) {
    match stmt {
        Stmt::Function { body, .. }
        | Stmt::While { body, .. }
        | Stmt::Foreach { body, .. }
        | Stmt::Block { body, .. }
        | Stmt::Lock { body, .. }
        | Stmt::Transaction { body, .. }
        | Stmt::Retry { body, .. }
        | Stmt::Worker { body, .. } => g_all(body, g),
        Stmt::ClassDef { members, .. } | Stmt::ServiceDecl { members, .. } => g_all(members, g),
        Stmt::If {
            then_branch,
            else_branch,
            ..
        } => {
            g_all(then_branch, g);
            if let Some(e) = else_branch {
                g_all(e, g);
            }
        }
        Stmt::DoOnFail {
            body, on_fail_body, ..
        } => {
            g_all(body, g);
            g_all(on_fail_body, g);
        }
        Stmt::Match { arms, .. } => {
            for arm in arms {
                g_all(&arm.body, g);
            }
        }
        Stmt::Import { .. }
        | Stmt::VarDecl { .. }
        | Stmt::ConstDecl { .. }
        | Stmt::DestructureDecl { .. }
        | Stmt::Expression { .. }
        | Stmt::Return { .. }
        | Stmt::Panic { .. }
        | Stmt::Fail { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Rollback { .. }
        | Stmt::Fork { .. }
        | Stmt::TypeDef { .. }
        | Stmt::EnumDef { .. }
        | Stmt::ListenerDecl { .. }
        | Stmt::AnnotationDecl { .. }
        | Stmt::Xmlns { .. } => {}
    }
}

fn g_all<G: FnMut(&Stmt)>(stmts: &[Stmt], g: &mut G) {
    for s in stmts {
        g(s);
    }
}

/// Applies `g` to each expression held directly by `stmt` (not by its nested
/// statements — `walk_stmts` reaches those).
fn for_each_direct_expr<G: FnMut(&Expr)>(stmt: &Stmt, g: &mut G) {
    match stmt {
        Stmt::VarDecl { initializer, .. } => {
            if let Some(e) = initializer {
                g(e);
            }
        }
        Stmt::ConstDecl { initializer, .. } => g(initializer),
        Stmt::DestructureDecl { initializer, .. } => g(initializer),
        Stmt::Expression { expression, .. } => g(expression),
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                g(e);
            }
        }
        Stmt::Panic { value, .. } | Stmt::Fail { value, .. } => g(value),
        Stmt::If { condition, .. } | Stmt::While { condition, .. } => g(condition),
        Stmt::Foreach { iterable, .. } => g(iterable),
        Stmt::Match { subject, arms, .. } => {
            g(subject);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    g(guard);
                }
            }
        }
        Stmt::EnumDef { members, .. } => {
            for m in members {
                if let Some(v) = &m.value {
                    g(v);
                }
            }
        }
        Stmt::Function { .. }
        | Stmt::ClassDef { .. }
        | Stmt::ServiceDecl { .. }
        | Stmt::Block { .. }
        | Stmt::Lock { .. }
        | Stmt::Transaction { .. }
        | Stmt::Retry { .. }
        | Stmt::Worker { .. }
        | Stmt::DoOnFail { .. }
        | Stmt::Import { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Rollback { .. }
        | Stmt::Fork { .. }
        | Stmt::TypeDef { .. }
        | Stmt::ListenerDecl { .. }
        | Stmt::AnnotationDecl { .. }
        | Stmt::Xmlns { .. } => {}
    }
}

/// Applies `g` to each expression directly nested in `expr` (one level).
fn for_each_child_expr<G: FnMut(&Expr)>(expr: &Expr, g: &mut G) {
    match expr {
        Expr::Binary { left, right, .. } => {
            g(left);
            g(right);
        }
        Expr::Unary { operand, .. } => g(operand),
        Expr::Grouping { expression, .. } => g(expression),
        Expr::Call {
            callee, arguments, ..
        } => {
            g(callee);
            g_all_exprs(arguments, g);
        }
        Expr::Assign { value, .. } => g(value),
        Expr::MemberAccess { object, member, .. } => {
            g(object);
            g(member);
        }
        Expr::FieldAccess { object, .. } => g(object),
        Expr::MemberAssign { target, value, .. } => {
            g(target);
            g(value);
        }
        Expr::MethodCall {
            object, arguments, ..
        }
        | Expr::RemoteCall {
            object, arguments, ..
        } => {
            g(object);
            g_all_exprs(arguments, g);
        }
        Expr::ArrayLiteral { elements, .. } => g_all_exprs(elements, g),
        Expr::MapLiteral { entries, .. } => {
            for (_k, v) in entries {
                g(v);
            }
        }
        Expr::Ternary {
            condition,
            true_expr,
            false_expr,
            ..
        } => {
            g(condition);
            g(true_expr);
            g(false_expr);
        }
        Expr::Elvis { expr, default, .. } => {
            g(expr);
            g(default);
        }
        Expr::Range { start, end, .. } => {
            g(start);
            g(end);
        }
        Expr::Cast { expr, .. }
        | Expr::Check { expr, .. }
        | Expr::TypeOf { expr, .. }
        | Expr::TypeTest { expr, .. } => g(expr),
        Expr::New { arguments, .. }
        | Expr::TableConstructor {
            rows: arguments, ..
        } => g_all_exprs(arguments, g),
        Expr::NamedArg { value, .. } => g(value),
        Expr::StringTemplate { interpolations, .. } => g_all_exprs(interpolations, g),
        Expr::Let { bindings, body, .. } => {
            for b in bindings {
                g(&b.value);
            }
            g(body);
        }
        Expr::Arrow { body, .. } => g(body),
        Expr::Start { call, .. } => g(call),
        Expr::Query { clauses, .. } => {
            for clause in clauses {
                match clause {
                    QueryClause::From { source, .. } => g(source),
                    QueryClause::Where(e) | QueryClause::Limit(e) | QueryClause::Select(e) => g(e),
                    QueryClause::Let(bindings) => {
                        for b in bindings {
                            g(&b.value);
                        }
                    }
                    QueryClause::Join {
                        source,
                        on_left,
                        on_right,
                        ..
                    } => {
                        g(source);
                        g(on_left);
                        g(on_right);
                    }
                    QueryClause::OrderBy(keys) => g_all_exprs(keys, g),
                    // Statements inside a `do` clause are reached by walk_stmts.
                    QueryClause::Do(_) | QueryClause::Other => {}
                }
            }
        }
        // Bodies are statement lists, reached by walk_stmts.
        Expr::AnonFunction { .. } | Expr::ObjectConstructor { .. } => {}
        Expr::Literal { .. } | Expr::Variable { .. } => {}
    }
}

fn g_all_exprs<G: FnMut(&Expr)>(exprs: &[Expr], g: &mut G) {
    for e in exprs {
        g(e);
    }
}
