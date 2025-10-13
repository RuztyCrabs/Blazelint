//! Semantic analysis for the Blazelint.
//!
//! This module walks the parser-produced abstract syntax tree, tracks lexical
//! scopes, and enforces the subset of Ballerina typing rules supported by the
//! linter. Each visitor emits structured diagnostics tagged with source spans
//! so the CLI can highlight offending code precisely.
use crate::ast::{BinaryOp, Expr, Literal, Stmt, UnaryOp};
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
}

impl Analyzer {
    /// Constructs a fresh analyzer with the root scope in place.
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            diagnostics: Vec::new(),
            current_function: None,
            functions: HashSet::new(),
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
                    .map(|ann| self.type_from_annotation(ann, span.clone()));

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
                    .map(|ann| self.type_from_annotation(ann, span.clone()));

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
                    .map(|ty| self.type_from_annotation(ty, name_span.clone()))
                    .unwrap_or(Type::Nil);

                let previous = self.current_function.take();
                self.current_function = Some(FunctionContext {
                    return_type: return_ty.clone(),
                });

                self.with_scope(|analyzer| {
                    for (param_name, ty_name) in params {
                        let param_type = analyzer.type_from_annotation(ty_name, name_span.clone());
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
        }
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
            UnaryOp::Minus => {
                if let Some(result) = self.numeric_operand(&operand_type) {
                    result
                } else {
                    if !operand_type.is_unknown() {
                        self.report(
                            span,
                            format!(
                                "Unary '-' expects numeric operand, found {}",
                                operand_type.description()
                            ),
                        );
                    }
                    Type::Unknown("unary".into())
                }
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
            BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Star => {
                if let Some(result) = self.numeric_result(&left_type, &right_type, false) {
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
                if let Some(result) = self.numeric_result(&left_type, &right_type, true) {
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
            BinaryOp::EqualEqual | BinaryOp::NotEqual => {
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
        if target == value {
            return true;
        }
        matches!((target, value), (Type::Float, Type::Int))
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
                if !self.functions.contains(name) {
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

    /// Converts a type annotation string into an internal `Type` value.
    fn type_from_annotation(&mut self, name: &str, span: Span) -> Type {
        match name {
            "int" => Type::Int,
            "float" => Type::Float,
            "boolean" => Type::Boolean,
            "string" => Type::String,
            "error" => Type::Error,
            "nil" => Type::Nil,
            "const" => Type::Unknown("const".to_string()),
            other => {
                self.report(span, format!("Unknown type '{other}'"));
                Type::Unknown(other.to_string())
            }
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
pub fn analyze(statements: &[Stmt]) -> Result<(), Vec<Diagnostic>> {
    Analyzer::new().analyze(statements)
}
