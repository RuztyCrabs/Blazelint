//! Semantic analysis for the Blazelint.
//!
//! This module walks the parser-produced abstract syntax tree, tracks lexical
//! scopes, and enforces the subset of Ballerina typing rules supported by the
//! linter. Each visitor emits structured diagnostics tagged with source spans
//! so the CLI can highlight offending code precisely.
use crate::ast::{BinaryOp, Expr, Literal, QueryClause, Stmt, TypeDescriptor, UnaryOp};
use crate::errors::{Diagnostic, DiagnosticKind, Span};
use std::collections::{HashMap, HashSet};

/// Internal representation of the types the analyzer understands.
#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    Boolean,
    String,
    Error,
    Nil,
    Array(Box<Type>),
    Map(Box<Type>),
    Unknown(String),
}

impl Type {
    /// Returns a human-readable name used in diagnostics and notes.
    fn description(&self) -> String {
        match self {
            Type::Int => "int".to_string(),
            Type::Float => "float".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::String => "string".to_string(),
            Type::Error => "error".to_string(),
            Type::Nil => "()".to_string(),
            Type::Array(elem) => format!("{}[]", elem.description()),
            Type::Map(val) => format!("map<{}>", val.description()),
            Type::Unknown(name) => name.clone(),
        }
    }

    /// Indicates whether the value arose from an unresolved or deferred type.
    fn is_unknown(&self) -> bool {
        matches!(self, Type::Unknown(_))
    }
}

/// Tracked metadata for a symbol bound in the current scope stack.
#[derive(Clone)]
pub struct Symbol {
    pub ty: Type,
    pub is_final: bool,
    pub is_const: bool,
    pub initialized: bool,
    pub declared_span: Span,
}

/// Context for the function currently being analyzed.
struct FunctionContext {
    return_type: Type,
}

/// Performs semantic validation over a sequence of statements.
pub struct Analyzer {
    scopes: Vec<HashMap<String, Symbol>>,
    diagnostics: Vec<Diagnostic>,
    current_function: Option<FunctionContext>,
    functions: HashSet<String>,
    imports: HashSet<String>,
    loop_depth: usize,
}

impl Analyzer {
    /// Constructs a fresh analyzer with the root scope in place.
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
            current_function: None,
            functions: HashSet::new(),
            imports: HashSet::new(),
            loop_depth: 0,
        }
    }

    /// Entry point used by the public `analyze` facade.
    fn analyze(mut self, stmts: &[Stmt]) -> Result<(), Vec<Diagnostic>> {
        self.collect_functions(stmts);
        for stmt in stmts {
            self.check_stmt(stmt);
        }
        if self.diagnostics.is_empty() {
            Ok(())
        } else {
            Err(self.diagnostics)
        }
    }

    /// Validates a single statement node and updates scope state as needed.
    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Import { package_path, .. } => {
                // Track imported module
                let module_name = package_path.last().unwrap_or(&String::new()).clone();
                self.imports.insert(module_name);
            }
            Stmt::VarDecl {
                is_final,
                name,
                name_span,
                type_annotation,
                initializer,
                span,
            } => {
                let declared_type = type_annotation
                    .as_ref()
                    .map(|ann| self.type_from_annotation(ann));

                if *is_final && initializer.is_none() {
                    self.report(
                        span.clone(),
                        format!("final variable '{name}' must be initialised"),
                    );
                }

                if let Some(existing) = self.current_scope().get(name) {
                    self.report(
                        name_span.clone(),
                        format!(
                            "Redeclaration of variable '{name}' (previously declared at {}..{})",
                            existing.declared_span.start, existing.declared_span.end
                        ),
                    );
                    return;
                }

                let mut symbol = Symbol {
                    ty: declared_type.clone().unwrap_or(Type::Unknown("var".into())),
                    is_final: *is_final,
                    is_const: false,
                    initialized: false,
                    declared_span: span.clone(),
                };

                if let Some(expr) = initializer {
                    let expr_type = self.check_expr(expr);
                    if let Some(declared) = declared_type {
                        if !Self::can_assign(&declared, &expr_type) {
                            self.report(
                                expr.span().clone(),
                                format!(
                                    "Type mismatch in initializer: expected {}, found {}",
                                    declared.description(),
                                    expr_type.description()
                                ),
                            );
                        }
                        symbol.ty = declared;
                    } else {
                        symbol.ty = expr_type;
                    }
                    symbol.initialized = true;
                }

                self.current_scope_mut().insert(name.clone(), symbol);
            }
            Stmt::ConstDecl {
                name,
                name_span,
                type_annotation,
                initializer,
                span,
            } => {
                let declared_type = type_annotation
                    .as_ref()
                    .map(|ann| self.type_from_annotation(ann));

                if let Some(existing) = self.current_scope().get(name) {
                    self.report(
                        name_span.clone(),
                        format!(
                            "Redeclaration of constant '{name}' (previously declared at {}..{})",
                            existing.declared_span.start, existing.declared_span.end
                        ),
                    );
                    return;
                }

                let mut symbol = Symbol {
                    ty: declared_type
                        .clone()
                        .unwrap_or(Type::Unknown("const".into())),
                    is_final: true,
                    is_const: true,
                    initialized: true,
                    declared_span: span.clone(),
                };

                let expr_type = self.check_expr(initializer);
                if let Some(declared) = declared_type {
                    if !Self::can_assign(&declared, &expr_type) {
                        self.report(
                            initializer.span().clone(),
                            format!(
                                "Type mismatch in initializer: expected {}, found {}",
                                declared.description(),
                                expr_type.description()
                            ),
                        );
                    }
                    symbol.ty = declared;
                } else {
                    symbol.ty = expr_type;
                }

                self.current_scope_mut().insert(name.clone(), symbol);
            }
            Stmt::Expression { expression, .. } => {
                self.check_expr(expression);
            }
            Stmt::Return { value, span } => {
                let expected = self
                    .current_function
                    .as_ref()
                    .map(|ctx| ctx.return_type.clone())
                    .unwrap_or(Type::Nil);

                match value {
                    Some(expr) => {
                        let value_type = self.check_expr(expr);
                        if !Self::can_assign(&expected, &value_type) {
                            self.report(
                                expr.span().clone(),
                                format!(
                                    "Type mismatch in return: expected {}, found {}",
                                    expected.description(),
                                    value_type.description()
                                ),
                            );
                        }
                    }
                    None => {
                        if expected != Type::Nil {
                            self.report(
                                span.clone(),
                                format!(
                                    "Missing return value: expected {}",
                                    expected.description()
                                ),
                            );
                        }
                    }
                }
            }
            Stmt::Panic { value, span } => {
                let value_type = self.check_expr(value);
                if value_type != Type::Error && !value_type.is_unknown() {
                    self.report(
                        span.clone(),
                        format!(
                            "panic expects expression of type error, found {}",
                            value_type.description()
                        ),
                    );
                }
            }
            Stmt::If {
                condition,
                then_branch,
                else_branch,
                ..
            } => {
                let condition_type = self.check_expr(condition);
                if condition_type != Type::Boolean && !condition_type.is_unknown() {
                    self.report(
                        condition.span().clone(),
                        format!(
                            "if condition must be boolean, found {}",
                            condition_type.description()
                        ),
                    );
                }
                self.with_scope(|analyzer| {
                    for stmt in then_branch {
                        analyzer.check_stmt(stmt);
                    }
                });
                if let Some(else_branch) = else_branch {
                    self.with_scope(|analyzer| {
                        for stmt in else_branch {
                            analyzer.check_stmt(stmt);
                        }
                    });
                }
            }
            Stmt::While {
                condition, body, ..
            } => {
                let condition_type = self.check_expr(condition);
                if condition_type != Type::Boolean && !condition_type.is_unknown() {
                    self.report(
                        condition.span().clone(),
                        format!(
                            "while condition must be boolean, found {}",
                            condition_type.description()
                        ),
                    );
                }
                self.loop_depth += 1;
                self.with_scope(|analyzer| {
                    for stmt in body {
                        analyzer.check_stmt(stmt);
                    }
                });
                self.loop_depth -= 1;
            }
            Stmt::Foreach {
                type_annotation,
                variable,
                extra_bindings,
                iterable,
                body,
                span: _,
            } => {
                let _iterable_type = self.check_expr(iterable);
                // TODO: Check that iterable is actually iterable

                self.loop_depth += 1;
                self.with_scope(|analyzer| {
                    let var_type = if let Some(type_ann) = type_annotation {
                        analyzer.type_from_annotation(type_ann)
                    } else {
                        Type::Unknown("foreach_var".to_string())
                    };

                    analyzer.current_scope_mut().insert(
                        variable.clone(),
                        Symbol {
                            ty: var_type,
                            is_final: true,
                            is_const: false,
                            initialized: true,
                            declared_span: iterable.span().clone(),
                        },
                    );
                    // Additional names bound by a destructuring foreach pattern.
                    for name in extra_bindings {
                        analyzer.current_scope_mut().insert(
                            name.clone(),
                            Symbol {
                                ty: Type::Unknown("foreach_var".to_string()),
                                is_final: true,
                                is_const: false,
                                initialized: true,
                                declared_span: iterable.span().clone(),
                            },
                        );
                    }

                    for stmt in body {
                        analyzer.check_stmt(stmt);
                    }
                });
                self.loop_depth -= 1;
            }
            Stmt::Break { span } => {
                if self.loop_depth == 0 {
                    self.report(span.clone(), "Break statement outside of loop".to_string());
                }
            }
            Stmt::Continue { span } => {
                if self.loop_depth == 0 {
                    self.report(
                        span.clone(),
                        "Continue statement outside of loop".to_string(),
                    );
                }
            }
            Stmt::Function {
                name: _,
                name_span,
                params,
                return_type,
                body,
                ..
            } => {
                let return_ty = return_type
                    .as_ref()
                    .map(|ty| self.type_from_annotation(ty))
                    .unwrap_or(Type::Nil);

                let previous = self.current_function.take();
                self.current_function = Some(FunctionContext {
                    return_type: return_ty.clone(),
                });

                self.with_scope(|analyzer| {
                    for (param_name, ty_name) in params {
                        let param_type = analyzer.type_from_annotation(ty_name);
                        analyzer.current_scope_mut().insert(
                            param_name.clone(),
                            Symbol {
                                ty: param_type,
                                is_final: true,
                                is_const: false,
                                initialized: true,
                                declared_span: name_span.clone(),
                            },
                        );
                    }
                    for stmt in body {
                        analyzer.check_stmt(stmt);
                    }
                });

                self.current_function = previous;
            }
            // Module-level declarations introduced in the grammar expansion.
            // Enum members are bound as usable symbols so references to them do not
            // report "undeclared variable". Other declarations are accepted without
            // deep analysis (parse-tolerant / deferred semantics).
            Stmt::EnumDef { members, .. } => {
                for member in members {
                    if let Some(value) = &member.value {
                        self.check_expr(value);
                    }
                    self.current_scope_mut().insert(
                        member.name.clone(),
                        Symbol {
                            ty: Type::Unknown("enum".to_string()),
                            is_final: true,
                            is_const: true,
                            initialized: true,
                            declared_span: member.name_span.clone(),
                        },
                    );
                }
            }
            Stmt::TypeDef { .. }
            | Stmt::ClassDef { .. }
            | Stmt::ServiceDecl { .. }
            | Stmt::ListenerDecl { .. }
            | Stmt::AnnotationDecl { .. }
            | Stmt::Xmlns { .. } => {}
            Stmt::Block { body, .. }
            | Stmt::Lock { body, .. }
            | Stmt::Transaction { body, .. }
            | Stmt::Retry { body, .. }
            | Stmt::Worker { body, .. } => {
                self.with_scope(|analyzer| {
                    for stmt in body {
                        analyzer.check_stmt(stmt);
                    }
                });
            }
            Stmt::Fail { value, .. } => {
                self.check_expr(value);
            }
            Stmt::DestructureDecl {
                names,
                name_spans,
                initializer,
                ..
            } => {
                self.check_expr(initializer);
                for (name, span) in names.iter().zip(name_spans.iter()) {
                    self.current_scope_mut().insert(
                        name.clone(),
                        Symbol {
                            ty: Type::Unknown("destructured".to_string()),
                            is_final: false,
                            is_const: false,
                            initialized: true,
                            declared_span: span.clone(),
                        },
                    );
                }
            }
            Stmt::Match { subject, arms, .. } => {
                self.check_expr(subject);
                for arm in arms {
                    self.with_scope(|analyzer| {
                        for name in &arm.bindings {
                            analyzer.current_scope_mut().insert(
                                name.clone(),
                                Symbol {
                                    ty: Type::Unknown("match_binding".to_string()),
                                    is_final: true,
                                    is_const: false,
                                    initialized: true,
                                    declared_span: 0..0,
                                },
                            );
                        }
                        if let Some(guard) = &arm.guard {
                            analyzer.check_expr(guard);
                        }
                        for stmt in &arm.body {
                            analyzer.check_stmt(stmt);
                        }
                    });
                }
            }
            Stmt::DoOnFail {
                body,
                on_fail_var,
                on_fail_body,
                ..
            } => {
                self.with_scope(|analyzer| {
                    for stmt in body {
                        analyzer.check_stmt(stmt);
                    }
                });
                self.with_scope(|analyzer| {
                    if let Some(var) = on_fail_var {
                        analyzer.current_scope_mut().insert(
                            var.clone(),
                            Symbol {
                                ty: Type::Error,
                                is_final: true,
                                is_const: false,
                                initialized: true,
                                declared_span: 0..0,
                            },
                        );
                    }
                    for stmt in on_fail_body {
                        analyzer.check_stmt(stmt);
                    }
                });
            }
            Stmt::Rollback { .. } | Stmt::Fork { .. } => {}
        }
    }

    /// Evaluates an expression and returns its inferred static type.
    fn check_expr(&mut self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal { value, .. } => self.type_from_literal(value),
            Expr::Variable { name, span } => self.lookup_variable(name, span.clone()),
            Expr::Grouping { expression, .. } => self.check_expr(expression),
            Expr::Unary { op, operand, span } => self.check_unary(op, operand, span.clone()),
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => self.check_binary(left, op, right, span.clone()),
            Expr::Assign { name, value, span } => {
                let rhs_type = self.check_expr(value);
                self.assign_variable(name, value.span().clone(), span.clone(), rhs_type)
            }
            Expr::Call {
                callee, arguments, ..
            } => self.check_call(callee, arguments),
            Expr::MemberAccess { object, member, .. } => {
                let obj_type = self.check_expr(object);
                self.check_expr(member);

                // Array/map access returns element type
                match obj_type {
                    Type::Array(elem_type) => *elem_type,
                    Type::Map(val_type) => *val_type,
                    Type::Unknown(_) => Type::Unknown("member_access".to_string()),
                    _ => {
                        self.report(
                            object.span().clone(),
                            format!("Cannot index type {}", obj_type.description()),
                        );
                        Type::Unknown("invalid_index".to_string())
                    }
                }
            }
            Expr::FieldAccess { object, .. } => {
                // Field types are not resolved yet; visit the object for its own
                // checks and treat the field value as unknown.
                let _obj_type = self.check_expr(object);
                Type::Unknown("field".to_string())
            }
            Expr::MemberAssign { target, value, .. } => {
                // Assignment to a field/index lvalue. Visit both sides; mutability
                // and type-compatibility checks on such targets are deferred.
                self.check_expr(target);
                self.check_expr(value)
            }
            Expr::MethodCall {
                object,
                method,
                arguments,
                ..
            } => {
                let obj_type = self.check_expr(object);
                for arg in arguments {
                    self.check_expr(arg);
                }

                // Common method type checking
                match (obj_type.clone(), method.as_str()) {
                    // Array methods
                    (Type::Array(_), "push") => Type::Nil,
                    (Type::Array(elem_type), "pop") => *elem_type,
                    (Type::Array(elem_type), "remove") => *elem_type,
                    (Type::Array(_), "length") => Type::Int,

                    // String methods
                    (Type::String, "length") => Type::Int,
                    (Type::String, "substring") => Type::String,
                    (Type::String, "toUpperCase") => Type::String,
                    (Type::String, "toLowerCase") => Type::String,

                    // Map methods
                    (Type::Map(val_type), "get") => *val_type,
                    (Type::Map(_), "keys") => Type::Array(Box::new(Type::String)),
                    (Type::Map(val_type), "values") => Type::Array(val_type),
                    (Type::Map(_), "length") => Type::Int,

                    _ => Type::Unknown("method_call".to_string()),
                }
            }
            Expr::ArrayLiteral { elements, .. } => {
                if elements.is_empty() {
                    return Type::Array(Box::new(Type::Unknown("empty_array".to_string())));
                }

                // A `[...]` constructor may be an array OR a tuple, so heterogeneous
                // elements are not an error. Still visit every element so variable
                // use/undeclared tracking runs across all of them.
                let first_type = self.check_expr(&elements[0]);
                let mut homogeneous = true;
                for elem in &elements[1..] {
                    let elem_type = self.check_expr(elem);
                    if !Self::can_assign(&first_type, &elem_type) {
                        homogeneous = false;
                    }
                }

                if homogeneous {
                    Type::Array(Box::new(first_type))
                } else {
                    // Mixed element types: treat as an (unresolved) tuple-like list.
                    Type::Unknown("list".to_string())
                }
            }
            Expr::MapLiteral { entries, .. } => {
                if entries.is_empty() {
                    return Type::Map(Box::new(Type::Unknown("empty_map".to_string())));
                }

                // A `{...}` constructor may be a map OR a record/mapping value, so
                // heterogeneous values are not an error. Visit every value for
                // use/undeclared tracking.
                let first_type = self.check_expr(&entries[0].1);
                let mut homogeneous = true;
                for (_key, value) in &entries[1..] {
                    let val_type = self.check_expr(value);
                    if !Self::can_assign(&first_type, &val_type) {
                        homogeneous = false;
                    }
                }

                if homogeneous {
                    Type::Map(Box::new(first_type))
                } else {
                    // Mixed value types: treat as an (unresolved) mapping value.
                    Type::Unknown("mapping".to_string())
                }
            }
            Expr::Ternary {
                condition,
                true_expr,
                false_expr,
                ..
            } => {
                let cond_type = self.check_expr(condition);
                if cond_type != Type::Boolean && !cond_type.is_unknown() {
                    self.report(
                        condition.span().clone(),
                        format!(
                            "Ternary condition must be boolean, found {}",
                            cond_type.description()
                        ),
                    );
                }
                let true_type = self.check_expr(true_expr);
                let false_type = self.check_expr(false_expr);
                // Return the type of the true branch, or unknown if they don't match
                if Self::can_assign(&true_type, &false_type) {
                    true_type
                } else {
                    Type::Unknown("ternary".to_string())
                }
            }
            Expr::Elvis { expr, default, .. } => {
                let expr_type = self.check_expr(expr);
                let default_type = self.check_expr(default);
                // Elvis operator returns the non-null value
                if Self::can_assign(&expr_type, &default_type) {
                    expr_type
                } else {
                    Type::Unknown("elvis".to_string())
                }
            }
            Expr::Range { start, end, .. } => {
                let _start_type = self.check_expr(start);
                let _end_type = self.check_expr(end);
                // TODO: Check that both are integers
                Type::Unknown("range".to_string())
            }
            Expr::Cast {
                type_desc, expr, ..
            } => {
                let _expr_type = self.check_expr(expr);
                self.type_from_annotation(type_desc)
            }
            Expr::New {
                type_desc,
                arguments,
                ..
            } => {
                for arg in arguments {
                    self.check_expr(arg);
                }
                match type_desc {
                    Some(desc) => self.type_from_annotation(desc),
                    None => Type::Unknown("object".to_string()),
                }
            }
            // `check`/`checkpanic` unwrap an error union; `trap` yields the value
            // or an error. Full union modelling is deferred, so return the inner
            // expression's type.
            Expr::Check { expr, .. } => self.check_expr(expr),
            Expr::TypeOf { expr, .. } => {
                self.check_expr(expr);
                Type::Unknown("typedesc".to_string())
            }
            Expr::TypeTest { expr, .. } => {
                self.check_expr(expr);
                Type::Boolean
            }
            // A backtick template may be a string, xml, or a tagged template
            // (`base16`/`base64` → byte[], `re` → regex). Its type is left
            // unresolved so assignments to any of these are accepted.
            Expr::StringTemplate { .. } => Type::Unknown("template".to_string()),
            Expr::Let { bindings, body, .. } => {
                self.scopes.push(HashMap::new());
                for binding in bindings {
                    let value_type = self.check_expr(&binding.value);
                    self.current_scope_mut().insert(
                        binding.name.clone(),
                        Symbol {
                            ty: value_type,
                            is_final: true,
                            is_const: false,
                            initialized: true,
                            declared_span: binding.value.span().clone(),
                        },
                    );
                }
                let body_type = self.check_expr(body);
                self.scopes.pop();
                body_type
            }
            Expr::AnonFunction {
                params,
                return_type,
                body,
                ..
            } => {
                let ret = return_type
                    .as_ref()
                    .map(|t| self.type_from_annotation(t))
                    .unwrap_or_else(|| Type::Unknown("infer".to_string()));
                let previous = self.current_function.take();
                self.current_function = Some(FunctionContext { return_type: ret });
                self.scopes.push(HashMap::new());
                for (param_name, param_ty) in params {
                    let ty = self.type_from_annotation(param_ty);
                    self.current_scope_mut().insert(
                        param_name.clone(),
                        Symbol {
                            ty,
                            is_final: true,
                            is_const: false,
                            initialized: true,
                            declared_span: 0..0,
                        },
                    );
                }
                for stmt in body {
                    self.check_stmt(stmt);
                }
                self.scopes.pop();
                self.current_function = previous;
                Type::Unknown("function".to_string())
            }
            Expr::Arrow { params, body, .. } => {
                self.scopes.push(HashMap::new());
                for param in params {
                    self.current_scope_mut().insert(
                        param.clone(),
                        Symbol {
                            ty: Type::Unknown("param".to_string()),
                            is_final: true,
                            is_const: false,
                            initialized: true,
                            declared_span: 0..0,
                        },
                    );
                }
                let _ = self.check_expr(body);
                self.scopes.pop();
                Type::Unknown("function".to_string())
            }
            Expr::RemoteCall {
                object, arguments, ..
            } => {
                self.check_expr(object);
                for arg in arguments {
                    self.check_expr(arg);
                }
                Type::Unknown("remote_call".to_string())
            }
            Expr::Query { clauses, .. } => {
                // Query clauses share one scope: bindings introduced by `from`,
                // `join`, and `let` are visible to later clauses (`where`,
                // `select`, ...).
                self.scopes.push(HashMap::new());
                for clause in clauses {
                    match clause {
                        QueryClause::From { vars, source } => {
                            self.check_expr(source);
                            for var in vars {
                                self.bind_query_var(var);
                            }
                        }
                        QueryClause::Where(expr)
                        | QueryClause::Limit(expr)
                        | QueryClause::Select(expr) => {
                            self.check_expr(expr);
                        }
                        QueryClause::Let(bindings) => {
                            for binding in bindings {
                                let ty = self.check_expr(&binding.value);
                                self.current_scope_mut().insert(
                                    binding.name.clone(),
                                    Symbol {
                                        ty,
                                        is_final: true,
                                        is_const: false,
                                        initialized: true,
                                        declared_span: binding.value.span().clone(),
                                    },
                                );
                            }
                        }
                        QueryClause::Join {
                            vars,
                            source,
                            on_left,
                            on_right,
                        } => {
                            self.check_expr(source);
                            for var in vars {
                                self.bind_query_var(var);
                            }
                            self.check_expr(on_left);
                            self.check_expr(on_right);
                        }
                        QueryClause::OrderBy(keys) => {
                            for key in keys {
                                self.check_expr(key);
                            }
                        }
                        QueryClause::Do(body) => {
                            for stmt in body {
                                self.check_stmt(stmt);
                            }
                        }
                        QueryClause::Other => {}
                    }
                }
                self.scopes.pop();
                Type::Unknown("query".to_string())
            }
            Expr::TableConstructor { rows, .. } => {
                for row in rows {
                    self.check_expr(row);
                }
                Type::Unknown("table".to_string())
            }
            Expr::Start { call, .. } => {
                self.check_expr(call);
                Type::Unknown("future".to_string())
            }
            // Object-constructor members are not deeply analyzed (deferred).
            Expr::ObjectConstructor { .. } => Type::Unknown("object".to_string()),
        }
    }

    /// Binds a query clause variable (from/join) into the current scope, ignoring
    /// empty names produced by leniently-consumed destructuring patterns.
    fn bind_query_var(&mut self, name: &str) {
        if name.is_empty() {
            return;
        }
        self.current_scope_mut().insert(
            name.to_string(),
            Symbol {
                ty: Type::Unknown("query_var".to_string()),
                is_final: true,
                is_const: false,
                initialized: true,
                declared_span: 0..0,
            },
        );
    }

    /// Enforces the operand rules for unary expressions.
    fn check_unary(&mut self, op: &UnaryOp, operand: &Expr, span: Span) -> Type {
        let operand_type = self.check_expr(operand);
        match op {
            UnaryOp::Bang => {
                if operand_type != Type::Boolean && !operand_type.is_unknown() {
                    self.report(
                        span,
                        format!(
                            "Unary '!' expects boolean operand, found {}",
                            operand_type.description()
                        ),
                    );
                }
                Type::Boolean
            }
            UnaryOp::Minus | UnaryOp::Plus => {
                if let Some(result) = self.numeric_operand(&operand_type) {
                    result
                } else {
                    if !operand_type.is_unknown() {
                        self.report(
                            span,
                            format!(
                                "Unary '-'/'+' expects numeric operand, found {}",
                                operand_type.description()
                            ),
                        );
                    }
                    Type::Unknown("unary".into())
                }
            }
            UnaryOp::BitwiseNot => {
                if operand_type != Type::Int && !operand_type.is_unknown() {
                    self.report(
                        span,
                        format!(
                            "Bitwise NOT expects integer operand, found {}",
                            operand_type.description()
                        ),
                    );
                }
                Type::Int
            }
        }
    }

    /// Applies operator-specific typing rules for binary expressions.
    fn check_binary(&mut self, left: &Expr, op: &BinaryOp, right: &Expr, span: Span) -> Type {
        let left_type = self.check_expr(left);
        let right_type = self.check_expr(right);

        if left_type.is_unknown() || right_type.is_unknown() {
            return Type::Unknown("binary".into());
        }

        match op {
            BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Star | BinaryOp::Percent => {
                // `+` also concatenates strings (and is valid on xml/other
                // sequence types, which resolve to Unknown and are accepted).
                if matches!(op, BinaryOp::Plus)
                    && left_type == Type::String
                    && right_type == Type::String
                {
                    Type::String
                } else if let Some(result) = self.numeric_result(&left_type, &right_type, false) {
                    result
                } else {
                    self.report(
                        span,
                        format!(
                            "Operator {:?} requires numeric operands, found {} and {}",
                            op,
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Unknown("binary".into())
                }
            }
            BinaryOp::Slash => {
                // Ballerina integer division: `int / int` yields `int`; a float
                // operand makes the result float.
                if let Some(result) = self.numeric_result(&left_type, &right_type, false) {
                    result
                } else {
                    self.report(
                        span,
                        format!(
                            "Operator '/' requires numeric operands, found {} and {}",
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Unknown("binary".into())
                }
            }
            BinaryOp::EqualEqual
            | BinaryOp::NotEqual
            | BinaryOp::EqualEqualEqual
            | BinaryOp::NotEqualEqual => {
                if self.can_compare(&left_type, &right_type) {
                    Type::Boolean
                } else {
                    self.report(
                        span,
                        format!(
                            "Equality comparison requires matching operand types, found {} and {}",
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Boolean
                }
            }
            BinaryOp::Greater | BinaryOp::GreaterEqual | BinaryOp::Less | BinaryOp::LessEqual => {
                if self
                    .numeric_result(&left_type, &right_type, false)
                    .is_some()
                {
                    Type::Boolean
                } else {
                    self.report(
                        span,
                        format!(
                            "Ordered comparison requires numeric operands, found {} and {}",
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Boolean
                }
            }
            BinaryOp::Is => {
                // Type checking operator - always returns boolean
                Type::Boolean
            }
            BinaryOp::And | BinaryOp::Or => {
                if left_type == Type::Boolean && right_type == Type::Boolean {
                    Type::Boolean
                } else {
                    self.report(
                        span,
                        format!(
                            "Logical operator requires boolean operands, found {} and {}",
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Boolean
                }
            }
            BinaryOp::BitwiseAnd | BinaryOp::BitwiseOr | BinaryOp::BitwiseXor => {
                if left_type == Type::Int && right_type == Type::Int {
                    Type::Int
                } else {
                    self.report(
                        span,
                        format!(
                            "Bitwise operator requires integer operands, found {} and {}",
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Int
                }
            }
            BinaryOp::LeftShift | BinaryOp::RightShift | BinaryOp::UnsignedRightShift => {
                if left_type == Type::Int && right_type == Type::Int {
                    Type::Int
                } else {
                    self.report(
                        span,
                        format!(
                            "Shift operator requires integer operands, found {} and {}",
                            left_type.description(),
                            right_type.description()
                        ),
                    );
                    Type::Int
                }
            }
            BinaryOp::PlusAssign | BinaryOp::MinusAssign => {
                // These are handled elsewhere in assignment context
                if let Some(result) = self.numeric_result(&left_type, &right_type, false) {
                    result
                } else {
                    Type::Unknown("compound_assign".into())
                }
            }
        }
    }

    /// Handles assignments, including mutability checks and type compatibility.
    fn assign_variable(
        &mut self,
        name: &str,
        value_span: Span,
        span: Span,
        rhs_type: Type,
    ) -> Type {
        if let Some(symbol) = self.lookup_symbol_mut(name) {
            let symbol_type = symbol.ty.clone();
            let issue = if symbol.is_const {
                Some((span.clone(), format!("Cannot assign to constant '{name}'")))
            } else if symbol.is_final && symbol.initialized {
                Some((
                    span.clone(),
                    format!("Cannot assign to final variable '{name}'"),
                ))
            } else if !Self::can_assign(&symbol_type, &rhs_type) {
                Some((
                    value_span.clone(),
                    format!(
                        "Type mismatch in assignment: expected {}, found {}",
                        symbol_type.description(),
                        rhs_type.description()
                    ),
                ))
            } else {
                symbol.initialized = true;
                None
            };

            if let Some((issue_span, message)) = issue {
                self.report(issue_span, message);
            }

            symbol_type
        } else {
            self.report(span, format!("Use of undeclared variable '{name}'"));
            Type::Unknown(name.to_string())
        }
    }

    /// Derives a type from a literal expression variant.
    fn type_from_literal(&self, literal: &Literal) -> Type {
        match literal {
            Literal::Boolean(_) => Type::Boolean,
            Literal::String(_) => Type::String,
            Literal::Nil => Type::Nil,
            Literal::Number(n) => {
                if (n.fract()).abs() < f64::EPSILON {
                    Type::Int
                } else {
                    Type::Float
                }
            }
        }
    }

    /// Returns the type for a numeric operand when the operator requires one.
    fn numeric_operand(&self, operand: &Type) -> Option<Type> {
        match operand {
            Type::Int => Some(Type::Int),
            Type::Float => Some(Type::Float),
            _ => None,
        }
    }

    /// Computes the resulting type for arithmetic expressions, if valid.
    fn numeric_result(&self, left: &Type, right: &Type, force_float: bool) -> Option<Type> {
        match (left, right) {
            (Type::Int, Type::Int) if !force_float => Some(Type::Int),
            (Type::Int, Type::Int) => Some(Type::Float),
            (Type::Float, Type::Float) => Some(Type::Float),
            (Type::Int, Type::Float) | (Type::Float, Type::Int) => Some(Type::Float),
            _ => None,
        }
    }

    /// Determines whether two operands can participate in an equality comparison.
    fn can_compare(&self, left: &Type, right: &Type) -> bool {
        matches!(
            (left, right),
            (Type::Int, Type::Int)
                | (Type::Float, Type::Float)
                | (Type::Boolean, Type::Boolean)
                | (Type::String, Type::String)
                | (Type::Int, Type::Float)
                | (Type::Float, Type::Int)
        )
    }

    /// Resolves an identifier reference, emitting diagnostics when undefined or uninitialised.
    fn lookup_variable(&mut self, name: &str, span: Span) -> Type {
        // `self` (enclosing object), `commit` (transaction action), and builtin
        // type names used as `typedesc` values are always available.
        if name == "self"
            || name == "commit"
            || matches!(
                name,
                "int"
                    | "string"
                    | "boolean"
                    | "float"
                    | "decimal"
                    | "byte"
                    | "anydata"
                    | "json"
                    | "xml"
                    | "any"
                    | "error"
            )
        {
            return Type::Unknown(name.to_string());
        }
        if let Some(symbol) = self.lookup_symbol(name).cloned() {
            if !symbol.initialized {
                self.report(
                    span.clone(),
                    format!("Variable '{name}' may be used before it is initialised"),
                );
            }
            symbol.ty
        } else {
            self.report(span, format!("Use of undeclared variable '{name}'"));
            Type::Unknown(name.to_string())
        }
    }

    /// Searches the scope stack for a symbol without taking ownership.
    fn lookup_symbol(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    /// Finds a mutable reference to a symbol in the scope stack, if present.
    fn lookup_symbol_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                return Some(symbol);
            }
        }
        None
    }

    /// Returns whether the analyzer permits assigning `value` into `target`.
    fn can_assign(target: &Type, value: &Type) -> bool {
        // Deferred/unresolved types (records, tuples, generics, user-defined
        // names, ...) resolve to `Unknown`. The linter cannot verify assignments
        // involving them, so it accepts rather than reports a false mismatch.
        if target.is_unknown() || value.is_unknown() {
            return true;
        }
        if target == value {
            return true;
        }
        // Recurse into container element types so, e.g., `Employee[]` (an array of
        // an unresolved element type) accepts an array-literal of mappings.
        match (target, value) {
            (Type::Array(t), Type::Array(v)) | (Type::Map(t), Type::Map(v)) => {
                Self::can_assign(t, v)
            }
            (Type::Float, Type::Int) => true,
            _ => false,
        }
    }

    /// Validates call expressions and, for now, records the callee type as unknown.
    fn check_call(&mut self, callee: &Expr, arguments: &[Expr]) -> Type {
        match callee {
            Expr::Variable {
                name,
                span: callee_span,
            } => {
                for arg in arguments {
                    self.check_expr(arg);
                }
                if name == "error" {
                    return Type::Error;
                }

                // Check for qualified call (module:function)
                if name.contains(':') {
                    let parts: Vec<&str> = name.split(':').collect();
                    if parts.len() == 2 {
                        let module = parts[0];
                        if self.imports.contains(module) {
                            // Valid imported function call
                            return Type::Unknown(format!("call:{name}"));
                        }
                    }
                }

                // A call target may be a function-typed variable in scope (a
                // closure/first-class function), not a declared function.
                if !self.functions.contains(name) && self.lookup_symbol(name).is_none() {
                    self.report(
                        callee_span.clone(),
                        format!("Call to unknown function '{name}'"),
                    );
                }
                Type::Unknown(format!("call:{name}"))
            }
            _ => {
                let _ = self.check_expr(callee);
                for arg in arguments {
                    self.check_expr(arg);
                }
                Type::Unknown("call".into())
            }
        }
    }

    /// Converts a type annotation/descriptor into an internal `Type` value.
    fn type_from_annotation(&mut self, type_desc: &TypeDescriptor) -> Type {
        match type_desc {
            TypeDescriptor::Basic(name) => match name.as_str() {
                "int" => Type::Int,
                "float" => Type::Float,
                "boolean" => Type::Boolean,
                "string" => Type::String,
                "decimal" => Type::Float, // Treat decimal as float for now
                "byte" => Type::Int,      // Treat byte as int for now
                "anydata" => Type::Unknown("anydata".to_string()),
                "error" => Type::Error,
                "nil" => Type::Nil,
                // Predeclared types (json, xml, ...) and user-defined type names
                // (records, classes, enums) are not fully resolved by the linter.
                // Treat them as unknown rather than flagging them as errors, so
                // real-world programs are not rejected.
                other => Type::Unknown(other.to_string()),
            },
            TypeDescriptor::Array { element_type, .. } => {
                let elem_ty = self.type_from_annotation(element_type);
                Type::Array(Box::new(elem_ty))
            }
            TypeDescriptor::Map { value_type } => {
                let val_ty = self.type_from_annotation(value_type);
                Type::Map(Box::new(val_ty))
            }
            TypeDescriptor::Optional(inner) => self.type_from_annotation(inner),
            TypeDescriptor::Union(types) => {
                if !types.is_empty() {
                    self.type_from_annotation(&types[0])
                } else {
                    Type::Unknown("union".to_string())
                }
            }
            // Parse-tolerant, deferred-semantics types: represented but not fully
            // type-checked yet. They resolve to Unknown so downstream checks stay
            // silent instead of producing false positives.
            TypeDescriptor::Tuple { .. } => Type::Unknown("tuple".to_string()),
            TypeDescriptor::Record { .. } => Type::Unknown("record".to_string()),
            TypeDescriptor::Object => Type::Unknown("object".to_string()),
            TypeDescriptor::Function { .. } => Type::Unknown("function".to_string()),
            TypeDescriptor::Generic { name, .. } => match name.as_str() {
                "error" => Type::Error,
                other => Type::Unknown(other.to_string()),
            },
            TypeDescriptor::Intersection(_) => Type::Unknown("intersection".to_string()),
            TypeDescriptor::Singleton(text) => Type::Unknown(text.clone()),
            TypeDescriptor::Distinct(inner) => self.type_from_annotation(inner),
            TypeDescriptor::Qualified { module, name } => Type::Unknown(format!("{module}:{name}")),
        }
    }

    /// Appends a semantic diagnostic covering the provided span.
    fn report(&mut self, span: Span, message: String) {
        self.diagnostics
            .push(Diagnostic::new(DiagnosticKind::Semantic, message, span));
    }

    /// Executes a closure with a new scope pushed on the stack.
    fn with_scope<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut Self),
    {
        self.scopes.push(HashMap::new());
        f(self);
        self.scopes.pop();
    }

    /// Collects function names ahead of time so undefined call targets can be reported.
    fn collect_functions(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            match stmt {
                Stmt::Function { name, body, .. } => {
                    self.functions.insert(name.clone());
                    self.collect_functions(body);
                }
                Stmt::If {
                    then_branch,
                    else_branch,
                    ..
                } => {
                    self.collect_functions(then_branch);
                    if let Some(else_branch) = else_branch {
                        self.collect_functions(else_branch);
                    }
                }
                _ => {}
            }
        }
    }

    /// Returns the current innermost scope.
    fn current_scope(&self) -> &HashMap<String, Symbol> {
        self.scopes.last().expect("at least one scope present")
    }

    /// Returns a mutable reference to the current innermost scope.
    fn current_scope_mut(&mut self) -> &mut HashMap<String, Symbol> {
        self.scopes.last_mut().expect("at least one scope present")
    }
}

/// Public facade used by the rest of the crate to run semantic analysis.
pub fn analyze(
    statements: &[Stmt],
    _line_tracker: &crate::utils::LineTracker,
) -> Result<(), Vec<Diagnostic>> {
    // For now, we're not using line_tracker in semantic analysis
    // but we accept it for future optimizations and consistency
    Analyzer::new().analyze(statements)
}
