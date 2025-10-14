//! Recursive-descent parser for the Blazelint front-end.
//!
//! The parser consumes the token stream emitted by the lexer and produces a
//! span-aware abstract syntax tree. It keeps precedence handling close to the
//! grammar specification so that follow-up stages can rely on predictable AST
//! shapes and accurate byte ranges for diagnostics.
use crate::ast::*;
use crate::errors::{ParseError, Span, Diagnostic};
use crate::lexer::Token;

/// Convenient alias for parser results carrying a `ParseError` on failure.
type ParseResult<T> = Result<T, ParseError>;

/// Stateful parser that walks the token list and builds AST nodes.
pub struct Parser {
    tokens: Vec<(usize, Token, usize)>,
    current: usize,
    errors: Vec<Diagnostic>,
}

impl Parser {
    /// Creates a parser over the provided token triples produced by the lexer.
    pub fn new(tokens: Vec<(usize, Token, usize)>) -> Self {
        Self { 
            tokens, 
            current: 0,
            errors: Vec::new(),
        }
    }

    /// Parses the entire token stream into a list of top-level statements.
    ///
    /// Returns a tuple of (statements, diagnostics). If diagnostics is non-empty,
    /// parsing encountered errors but attempted to continue. Statements may be
    /// partial or empty in case of severe syntax errors.
    pub fn parse(mut self) -> (Vec<Stmt>, Vec<Diagnostic>) {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            match self.declaration() {
                Ok(stmt) => statements.push(stmt),
                Err(err) => {
                    // Convert ParseError to Diagnostic and collect it
                    self.errors.push(err.into());
                    // Synchronize to recover from error
                    self.synchronize();
                }
            }
        }
        (statements, self.errors)
    }
    
    /// Synchronizes the parser state after an error by advancing to the next
    /// statement boundary. This allows the parser to recover and continue
    /// finding more errors instead of stopping at the first one.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            // If we just passed a semicolon, we're at a statement boundary
            if matches!(self.previous(), Some(Token::Semicolon)) {
                return;
            }
            
            // If we see a keyword that starts a new statement/declaration, stop
            match self.peek() {
                Some(Token::Function) 
                | Some(Token::Public)
                | Some(Token::Import)
                | Some(Token::If)
                | Some(Token::While)
                | Some(Token::Foreach)
                | Some(Token::Return)
                | Some(Token::Const) => return,
                _ => {
                    self.advance().ok();
                }
            }
        }
    }

    /// Parses a top-level declaration (variable, function, or statement).
    fn declaration(&mut self) -> ParseResult<Stmt> {
        if self.match_token(&[Token::Import])? {
            self.import_declaration()
        } else if self.starts_var_decl() || matches!(self.peek(), Some(Token::Const)) {
            self.var_decl()
        } else if matches!(self.peek(), Some(Token::Public | Token::Function)) {
            self.function()
        } else {
            self.statement()
        }
    }

    /// Parses an import declaration (import ballerina/io;).
    fn import_declaration(&mut self) -> ParseResult<Stmt> {
        let import_span_start = self.previous_span().start;
        
        let mut package_path = Vec::new();
        let first_token = self.advance_owned()?;
        match first_token {
            Token::Identifier(name) => package_path.push(name),
            _ => return Err(self.error_previous("Expected package name after 'import'", Some("identifier"))),
        }
        
        while self.match_token(&[Token::Slash])? {
            let next_token = self.advance_owned()?;
            match next_token {
                Token::Identifier(name) => package_path.push(name),
                _ => return Err(self.error_previous("Expected package component after '/'", Some("identifier"))),
            }
        }
        
        self.consume(Token::Semicolon, "Expected ';' after import", Some("';'"))?;
        let semicolon_span = self.previous_span();
        
        Ok(Stmt::Import {
            package_path,
            span: import_span_start..semicolon_span.end,
        })
    }

    /// Parses a `var` declaration and optional type/initializer pair.
    fn var_decl(&mut self) -> ParseResult<Stmt> {
        let mut span_start = self.current_span().start;

        if self.match_token(&[Token::Const])? {
            if Self::is_type_start(self.peek().unwrap()) {
                return Err(
                    self.error_previous("const declarations cannot have a type annotation", None)
                );
            }

            let name_token = self.advance_owned()?;
            let name = match name_token {
                Token::Identifier(name) => name,
                _ => {
                    return Err(self.error_previous(
                        "Expected constant name after 'const'",
                        Some("identifier"),
                    ));
                }
            };
            let name_span = self.previous_span();

            self.consume(
                Token::Eq,
                "Constant declarations must be initialized",
                Some("'='"),
            )?;
            let initializer = self.expression()?;

            self.consume(
                Token::Semicolon,
                "Expected ';' after constant declaration",
                Some("';'"),
            )?;

            let semicolon_span = self.previous_span();
            let decl_span = span_start.min(name_span.start)..semicolon_span.end;

            return Ok(Stmt::ConstDecl {
                name,
                name_span,
                type_annotation: None,
                initializer,
                span: decl_span,
            });
        }

        let mut is_final = false;
        if self.match_token(&[Token::Final])? {
            is_final = true;
            span_start = self.previous_span().start;
        }

        let uses_var_keyword = self.match_token(&[Token::Var])?;
        if uses_var_keyword {
            span_start = span_start.min(self.previous_span().start);
        }

        let (name, name_span, type_annotation, initializer) = if uses_var_keyword {
            let name_token = self.advance_owned()?;
            let ident = match name_token {
                Token::Identifier(name) => name,
                _ => {
                    return Err(self
                        .error_previous("Expected variable name after 'var'", Some("identifier")))
                }
            };
            let name_span = self.previous_span();

            self.consume(
                Token::Eq,
                "Variables declared with 'var' must include an initializer",
                Some("'='"),
            )?;
            let expr = self.expression()?;

            (ident, name_span, None, Some(expr))
        } else {
            let type_desc = self.parse_type_descriptor()?;
            let name_token = self.advance_owned()?;
            let ident = match name_token {
                Token::Identifier(name) => name,
                _ => {
                    return Err(self.error_previous(
                        "Expected variable name after type descriptor",
                        Some("identifier"),
                    ))
                }
            };
            let name_span = self.previous_span();

            let initializer = if self.match_token(&[Token::Eq])? {
                Some(self.expression()?)
            } else {
                None
            };

            (ident, name_span, Some(type_desc), initializer)
        };

        self.consume(
            Token::Semicolon,
            "Expected ';' after variable declaration",
            Some("';'"),
        )?;
        let semicolon_span = self.previous_span();
        let mut decl_span = span_start.min(name_span.start)..semicolon_span.end;
        if let Some(ref init_expr) = initializer {
            let init_span = init_expr.span();
            decl_span = decl_span.start.min(init_span.start)..decl_span.end.max(init_span.end);
        }

        Ok(Stmt::VarDecl {
            is_final,
            name,
            name_span,
            type_annotation,
            initializer,
            span: decl_span,
        })
    }

    /// Parses a single statement (if, return, panic, or expression).
    fn statement(&mut self) -> ParseResult<Stmt> {
        match self.peek() {
            Some(Token::If) => self.if_statement(),
            Some(Token::While) => self.while_statement(),
            Some(Token::Foreach) => self.foreach_statement(),
            Some(Token::Break) => {
                self.advance()?;
                let span = self.previous_span();
                self.consume(Token::Semicolon, "Expected ';' after break", Some("';'"))?;
                Ok(Stmt::Break { span })
            }
            Some(Token::Continue) => {
                self.advance()?;
                let span = self.previous_span();
                self.consume(Token::Semicolon, "Expected ';' after continue", Some("';'"))?;
                Ok(Stmt::Continue { span })
            }
            Some(Token::Return) => {
                self.advance()?;
                let keyword_span = self.previous_span();
                let expr = if self.check(&Token::Semicolon) {
                    None
                } else {
                    Some(self.expression()?)
                };
                self.consume(Token::Semicolon, "Expected ';' after return", Some("';'"))?;
                let semicolon_span = self.previous_span();
                let span_end = expr
                    .as_ref()
                    .map(|e| e.span().end)
                    .unwrap_or(keyword_span.end);
                let span = keyword_span.start..semicolon_span.end.max(span_end);
                Ok(Stmt::Return { value: expr, span })
            }
            Some(Token::Panic) => {
                self.advance()?;
                let expr = self.expression()?;
                let keyword_span = self.previous_span();
                self.consume(Token::Semicolon, "Expected ';' after panic", Some("';'"))?;
                let semicolon_span = self.previous_span();
                let span = keyword_span.start.min(expr.span().start)..semicolon_span.end;
                Ok(Stmt::Panic { value: expr, span })
            }
            _ => {
                let expr = self.expression()?;
                self.consume(
                    Token::Semicolon,
                    "Expected ';' after expression",
                    Some("';'"),
                )?;
                let semicolon_span = self.previous_span();
                let span = expr.span().start..semicolon_span.end;
                Ok(Stmt::Expression {
                    expression: expr,
                    span,
                })
            }
        }
    }

    /// Parses an `if`/`else` statement and its associated blocks.
    fn if_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'if'
        let if_span = self.previous_span();
        self.consume(Token::LParen, "Expected '(' after 'if'", Some("'('"))?;
        let condition = self.expression()?;
        self.consume(Token::RParen, "Expected ')' after condition", Some("')'"))?;

        self.consume(Token::LBrace, "Expected '{' before then block", Some("'{'"))?;
        let then_block = self.block()?;
        let mut span_end = self.previous_span().end;
        let else_block = if self.match_token(&[Token::Else])? {
            // Check for else if
            if self.check(&Token::If) {
                // Parse else if as a single if statement
                let else_if_stmt = self.if_statement()?;
                span_end = else_if_stmt.span().end;
                Some(vec![else_if_stmt])
            } else {
                self.consume(Token::LBrace, "Expected '{' before else block", Some("'{'"))?;
                let else_block = self.block()?;
                span_end = self.previous_span().end;
                Some(else_block)
            }
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch: then_block,
            else_branch: else_block,
            span: if_span.start..span_end,
        })
    }

    /// Parses a while loop statement.
    fn while_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'while'
        let while_span = self.previous_span();
        self.consume(Token::LParen, "Expected '(' after 'while'", Some("'('"))?;
        let condition = self.expression()?;
        self.consume(Token::RParen, "Expected ')' after condition", Some("')'"))?;
        self.consume(Token::LBrace, "Expected '{' before while body", Some("'{'"))?;
        let body = self.block()?;
        let span_end = self.previous_span().end;
        Ok(Stmt::While {
            condition,
            body,
            span: while_span.start..span_end,
        })
    }

    /// Parses a foreach loop statement.
    fn foreach_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'foreach'
        let foreach_span = self.previous_span();
        
        // Parse optional type annotation
        let type_annotation = if Self::is_type_start(&self.peek().cloned().unwrap_or(Token::Semicolon)) {
            Some(self.parse_type_descriptor()?)
        } else {
            None
        };
        
        // Parse variable name
        let var_token = self.advance_owned()?;
        let variable = match var_token {
            Token::Identifier(name) => name,
            _ => return Err(self.error_previous("Expected variable name in foreach", Some("identifier"))),
        };
        
        self.consume(Token::In, "Expected 'in' after foreach variable", Some("'in'"))?;
        let iterable = self.expression()?;
        self.consume(Token::LBrace, "Expected '{' before foreach body", Some("'{'"))?;
        let body = self.block()?;
        let span_end = self.previous_span().end;
        
        Ok(Stmt::Foreach {
            type_annotation,
            variable,
            iterable,
            body,
            span: foreach_span.start..span_end,
        })
    }

    /// Parses a block enclosed in `{}` and returns its nested statements.
    fn block(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            stmts.push(self.declaration()?);
        }
        self.consume(Token::RBrace, "Expected '}' at end of block", Some("'}'"))?;
        Ok(stmts)
    }

    /// Parses a `function` declaration including parameters, optional return type, and body.
    fn function(&mut self) -> ParseResult<Stmt> {
        let is_public = self.match_token(&[Token::Public])?;
        
        self.advance()?; // consume 'function'
        let keyword_span = self.previous_span();
        let name_token = self.advance_owned()?;
        let name_span = self.previous_span();
        let name = match name_token {
            Token::Identifier(n) => n,
            _ => return Err(self.error_previous("Expected function name", Some("identifier"))),
        };

        self.consume(
            Token::LParen,
            "Expected '(' after function name",
            Some("'('"),
        )?;
        let mut params = Vec::new();
        while !self.check(&Token::RParen) {
            // Parse type first, then parameter name
            let param_type = self.parse_type_descriptor()?;
            let param_token = self.advance_owned()?;
            let param_name = match param_token {
                Token::Identifier(name) => name,
                _ => return Err(self.error_previous("Expected parameter name", Some("identifier"))),
            };
            params.push((param_name, param_type));
            if !self.check(&Token::RParen) {
                self.consume(Token::Comma, "Expected ',' between parameters", Some("','"))?;
            }
        }
        self.consume(Token::RParen, "Expected ')' after parameters", Some("')'"))?;

        let return_type = if self.match_token(&[Token::Returns])? {
            Some(self.parse_type_descriptor()?)
        } else {
            None
        };

        self.consume(
            Token::LBrace,
            "Expected '{' before function body",
            Some("'{'"),
        )?;
        let body = self.block()?;
        let body_end_span = self.previous_span();
        Ok(Stmt::Function {
            is_public,
            name,
            name_span,
            params,
            return_type,
            body,
            span: keyword_span.start..body_end_span.end,
        })
    }

    /// Parses an expression entry point.
    fn expression(&mut self) -> ParseResult<Expr> {
        self.assignment()
    }

    /// Parses an assignment expression, returning an error for invalid targets.
    fn assignment(&mut self) -> ParseResult<Expr> {
        let expr = self.ternary()?;

        if self.match_token(&[Token::Eq, Token::PlusEq, Token::MinusEq])? {
            let op_token = self.previous().cloned().expect("assignment operator");
            let assign_span = self.previous_span();
            let value = self.assignment()?;
            let value_span_end = value.span().end;

            if let Expr::Variable {
                name,
                span: name_span,
            } = expr
            {
                let span_start = name_span.start.min(assign_span.start);
                let span_end = value_span_end.max(assign_span.end);
                
                let op = match op_token {
                    Token::Eq => None,
                    Token::PlusEq => Some(BinaryOp::PlusAssign),
                    Token::MinusEq => Some(BinaryOp::MinusAssign),
                    _ => unreachable!(),
                };
                
                // For compound assignment, treat as binary op
                let final_value = if let Some(binop) = op {
                    Box::new(Expr::Binary {
                        left: Box::new(Expr::Variable {
                            name: name.clone(),
                            span: name_span.clone(),
                        }),
                        op: binop,
                        right: Box::new(value),
                        span: name_span.start..value_span_end,
                    })
                } else {
                    Box::new(value)
                };
                
                return Ok(Expr::Assign {
                    name,
                    value: final_value,
                    span: span_start..span_end,
                });
            }

            return Err(ParseError::new(
                "Invalid assignment target",
                assign_span,
                Some("identifier"),
            ));
        }

        Ok(expr)
    }

    /// Parses ternary and elvis operators (`? :`, `?:`).
    fn ternary(&mut self) -> ParseResult<Expr> {
        let mut expr = self.logic_or()?;

        if self.match_token(&[Token::QuestionColon])? {
            // Elvis operator: expr ?: default
            let span_start = expr.span().start;
            let default = self.logic_or()?;
            let span_end = default.span().end;
            expr = Expr::Elvis {
                expr: Box::new(expr),
                default: Box::new(default),
                span: span_start..span_end,
            };
        } else if self.match_token(&[Token::Question])? {
            // Ternary operator: condition ? true_expr : false_expr
            let span_start = expr.span().start;
            let true_expr = self.expression()?;
            self.consume(Token::Colon, "Expected ':' in ternary expression", Some("':'"))?;
            let false_expr = self.ternary()?;
            let span_end = false_expr.span().end;
            expr = Expr::Ternary {
                condition: Box::new(expr),
                true_expr: Box::new(true_expr),
                false_expr: Box::new(false_expr),
                span: span_start..span_end,
            };
        }

        Ok(expr)
    }

    /// Parses a logical OR expression (`||`).
    fn logic_or(&mut self) -> ParseResult<Expr> {
        let mut expr = self.logic_and()?;

        while self.match_token(&[Token::PipePipe])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.logic_and()?;
            let op = match op_token {
                Token::PipePipe => BinaryOp::Or,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses a logical AND expression (`&&`).
    fn logic_and(&mut self) -> ParseResult<Expr> {
        let mut expr = self.equality()?;

        while self.match_token(&[Token::AmpAmp])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.equality()?;
            let op = match op_token {
                Token::AmpAmp => BinaryOp::And,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses an equality comparison (`==` / `!=`).
    fn equality(&mut self) -> ParseResult<Expr> {
        let mut expr = self.comparison()?;

        while self.match_token(&[Token::EqEq, Token::BangEq])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.comparison()?;
            let op = match op_token {
                Token::EqEq => BinaryOp::EqualEqual,
                Token::BangEq => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses an ordered comparison (`>`, `>=`, `<`, `<=`).
    fn comparison(&mut self) -> ParseResult<Expr> {
        let mut expr = self.shift()?;

        while self.match_token(&[Token::Gt, Token::Ge, Token::Lt, Token::Le])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.shift()?;
            let op = match op_token {
                Token::Gt => BinaryOp::Greater,
                Token::Ge => BinaryOp::GreaterEqual,
                Token::Lt => BinaryOp::Less,
                Token::Le => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses shift expressions (`<<`, `>>`, `>>>`).
    fn shift(&mut self) -> ParseResult<Expr> {
        let mut expr = self.term()?;

        while self.match_token(&[Token::LtLt, Token::GtGt, Token::GtGtGt])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.term()?;
            let op = match op_token {
                Token::LtLt => BinaryOp::LeftShift,
                Token::GtGt => BinaryOp::RightShift,
                Token::GtGtGt => BinaryOp::UnsignedRightShift,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses an additive expression (`+`, `-`).
    fn term(&mut self) -> ParseResult<Expr> {
        let mut expr = self.bitwise()?;

        while self.match_token(&[Token::Plus, Token::Minus])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.bitwise()?;
            let op = match op_token {
                Token::Plus => BinaryOp::Plus,
                Token::Minus => BinaryOp::Minus,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses bitwise expressions (`&`, `|`, `^`).
    fn bitwise(&mut self) -> ParseResult<Expr> {
        let mut expr = self.factor()?;

        while self.match_token(&[Token::Amp, Token::Pipe, Token::Caret])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.factor()?;
            let op = match op_token {
                Token::Amp => BinaryOp::BitwiseAnd,
                Token::Pipe => BinaryOp::BitwiseOr,
                Token::Caret => BinaryOp::BitwiseXor,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses a multiplicative expression (`*`, `/`, `%`).
    fn factor(&mut self) -> ParseResult<Expr> {
        let mut expr = self.unary()?;

        while self.match_token(&[Token::Star, Token::Slash, Token::Percent])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.unary()?;
            let op = match op_token {
                Token::Star => BinaryOp::Star,
                Token::Slash => BinaryOp::Slash,
                Token::Percent => BinaryOp::Percent,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses a unary expression (`!`, unary `-`, `+`, `~`).
    fn unary(&mut self) -> ParseResult<Expr> {
        if self.match_token(&[Token::Bang, Token::Minus, Token::Plus, Token::Tilde])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let op = match op_token {
                Token::Bang => UnaryOp::Bang,
                Token::Minus => UnaryOp::Minus,
                Token::Plus => UnaryOp::Plus,
                Token::Tilde => UnaryOp::BitwiseNot,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            return Ok(self.make_unary_expr(op, op_span, right));
        }

        self.call()
    }

    /// Parses postfix function-call chains and member access.
    fn call(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.match_token(&[Token::LParen])? {
                let open_span = self.previous_span();
                expr = self.finish_call(expr, open_span)?;
            } else if self.match_token(&[Token::Dot])? {
                let method_token = self.advance_owned()?;
                let method_name = match method_token {
                    Token::Identifier(name) => name,
                    _ => return Err(self.error_previous("Expected method name after '.'", Some("identifier"))),
                };
                
                if self.match_token(&[Token::LParen])? {
                    // Method call: obj.method()
                    let mut arguments = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            arguments.push(self.expression()?);
                            if !self.match_token(&[Token::Comma])? {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RParen, "Expected ')' after arguments", Some("')'"))?;
                    let close_span = self.previous_span();
                    let span = expr.span().start..close_span.end;
                    expr = Expr::MethodCall {
                        object: Box::new(expr),
                        method: method_name,
                        arguments,
                        span,
                    };
                } else {
                    // Field access: obj.field (treat as variable for now)
                    let span = expr.span().start..self.previous_span().end;
                    expr = Expr::MemberAccess {
                        object: Box::new(expr),
                        member: Box::new(Expr::Variable {
                            name: method_name,
                            span: self.previous_span(),
                        }),
                        span,
                    };
                }
            } else if self.match_token(&[Token::LBracket])? {
                // Array/map access: obj[index]
                let index = self.expression()?;
                self.consume(Token::RBracket, "Expected ']' after index", Some("']'"))?;
                let close_span = self.previous_span();
                let span = expr.span().start..close_span.end;
                expr = Expr::MemberAccess {
                    object: Box::new(expr),
                    member: Box::new(index),
                    span,
                };
            } else if self.check(&Token::Colon) {
                // Check if this is a qualified call: module:function(...)
                // Only parse as qualified call if we have identifier:identifier pattern
                if let Expr::Variable { .. } = expr {
                    if matches!(self.peek_n(1), Some(Token::Identifier(_))) {
                        self.advance()?; // consume colon
                        
                        // Qualified call: module:function()
                        let func_token = self.advance_owned()?;
                        let func_name = match func_token {
                            Token::Identifier(name) => name,
                            _ => return Err(self.error_previous("Expected function name after ':'", Some("identifier"))),
                        };
                        
                        // Build qualified name: module:function
                        let (module_name, span_start) = match &expr {
                            Expr::Variable { name, span } => (name.clone(), span.start),
                            _ => return Err(self.error_previous("Qualified calls require module name before ':'", None)),
                        };
                        let qualified_name = format!("{}:{}", module_name, func_name);
                
                if self.match_token(&[Token::LParen])? {
                    let mut arguments = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            arguments.push(self.expression()?);
                            if !self.match_token(&[Token::Comma])? {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RParen, "Expected ')' after arguments", Some("')'"))?;
                    let close_span = self.previous_span();
                    expr = Expr::Call {
                        callee: Box::new(Expr::Variable {
                            name: qualified_name,
                            span: span_start..close_span.start,
                        }),
                        arguments,
                        span: span_start..close_span.end,
                    };
                } else {
                    // Just module:function reference without call
                    let span = span_start..self.previous_span().end;
                    expr = Expr::Variable {
                        name: qualified_name,
                        span,
                    };
                }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        Ok(expr)
    }

    /// Collects zero or more arguments after the opening parenthesis of a call.
    fn finish_call(&mut self, callee: Expr, open_span: Span) -> ParseResult<Expr> {
        let mut arguments = Vec::new();
        if !self.check(&Token::RParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
        }
        self.consume(Token::RParen, "Expected ')' after arguments", Some("')'"))?;
        let close_span = self.previous_span();
        Ok(self.make_call_expr(callee, arguments, open_span, close_span))
    }

    /// Parses a primary expression (literals, identifiers, or grouped subexpressions).
    fn primary(&mut self) -> ParseResult<Expr> {
        let token = self.advance_owned()?;
        let token_span = self.previous_span();
        match token {
            Token::True => Ok(self.make_literal_expr(Literal::Boolean(true), token_span)),
            Token::False => Ok(self.make_literal_expr(Literal::Boolean(false), token_span)),
            Token::Number(n) => Ok(self.make_literal_expr(Literal::Number(n), token_span)),
            Token::StringLiteral(s) => Ok(self.make_literal_expr(Literal::String(s), token_span)),
            Token::StringTemplate(s) => Ok(self.make_literal_expr(Literal::String(s), token_span)), // Treat templates as strings for now
            Token::Identifier(name) => {
                // Check for type cast: identifier followed by backtick is `type `template``
                if matches!(self.peek(), Some(Token::StringTemplate(_))) {
                    // This is a type cast of template string
                    let template_token = self.advance_owned()?;
                    if let Token::StringTemplate(s) = template_token {
                        let end_span = self.previous_span();
                        Ok(Expr::Cast {
                            type_desc: TypeDescriptor::Basic(name),
                            expr: Box::new(self.make_literal_expr(Literal::String(s), end_span.clone())),
                            span: token_span.start..end_span.end,
                        })
                    } else {
                        unreachable!()
                    }
                } else {
                    Ok(Expr::Variable {
                        name,
                        span: token_span,
                    })
                }
            }
            Token::LParen => {
                let open_span = token_span;
                // Check for nil literal: ()
                if self.check(&Token::RParen) {
                    self.advance()?;
                    let close_span = self.previous_span();
                    return Ok(self.make_literal_expr(Literal::Nil, open_span.start..close_span.end));
                }
                let expr = self.expression()?;
                self.consume(Token::RParen, "Expected ')' after expression", Some("')'"))?;
                let close_span = self.previous_span();
                Ok(self.make_grouping_expr(open_span, expr, close_span))
            }
            Token::Lt => {
                // Type cast: <type> expression
                let type_desc = self.parse_type_descriptor()?;
                self.consume(Token::Gt, "Expected '>' after cast type", Some("'>'"))?;
                let expr = self.unary()?;
                let end_span = expr.span().clone();
                Ok(Expr::Cast {
                    type_desc,
                    expr: Box::new(expr),
                    span: token_span.start..end_span.end,
                })
            }
            Token::LBracket => {
                // Array literal: [1, 2, 3]
                let open_span = token_span;
                let mut elements = Vec::new();
                
                if !self.check(&Token::RBracket) {
                    loop {
                        elements.push(self.expression()?);
                        if !self.match_token(&[Token::Comma])? {
                            break;
                        }
                    }
                }
                
                self.consume(Token::RBracket, "Expected ']' after array elements", Some("']'"))?;
                let close_span = self.previous_span();
                
                Ok(Expr::ArrayLiteral {
                    elements,
                    span: open_span.start..close_span.end,
                })
            }
            Token::LBrace => {
                // Map literal: {key: value}
                let open_span = token_span;
                let mut entries = Vec::new();
                
                if !self.check(&Token::RBrace) {
                    loop {
                        let key_token = self.advance_owned()?;
                        let key = match key_token {
                            Token::StringLiteral(s) => s,
                            Token::Identifier(s) => s,
                            _ => return Err(self.error_previous("Expected string key in map literal", Some("string"))),
                        };
                        
                        self.consume(Token::Colon, "Expected ':' after map key", Some("':'"))?;
                        let value = self.expression()?;
                        entries.push((key, value));
                        
                        if !self.match_token(&[Token::Comma])? {
                            break;
                        }
                    }
                }
                
                self.consume(Token::RBrace, "Expected '}' after map entries", Some("'}'"))?;
                let close_span = self.previous_span();
                
                Ok(Expr::MapLiteral {
                    entries,
                    span: open_span.start..close_span.end,
                })
            }
            _ => Err(self.error_previous(
                &format!("Unexpected token in expression: {:?}", token),
                None,
            )),
        }
    }

    /// Parses a type annotation following the limited Ballerina subset grammar.
    fn parse_type(&mut self) -> ParseResult<String> {
        let token = self.advance_owned()?;
        match token {
            Token::Identifier(s) => Ok(s),
            Token::Int => Ok("int".to_string()),
            Token::String => Ok("string".to_string()),
            Token::Boolean => Ok("boolean".to_string()),
            Token::Float => Ok("float".to_string()),
            t => Err(self.error_previous(&format!("Expected type, found {:?}", t), Some("type"))),
        }
    }

    /// Parses a type descriptor with array suffixes, maps, and other complex types.
    fn parse_type_descriptor(&mut self) -> ParseResult<TypeDescriptor> {
        let mut type_desc = if self.match_token(&[Token::Map])? {
            self.consume(Token::Lt, "Expected '<' after 'map'", Some("'<'"))?;
            let value_type = Box::new(self.parse_type_descriptor()?);
            self.consume(Token::Gt, "Expected '>' after map value type", Some("'>'"))?;
            TypeDescriptor::Map { value_type }
        } else {
            let token = self.advance_owned()?;
            let base_type = match token {
                Token::Int => "int".to_string(),
                Token::String => "string".to_string(),
                Token::Boolean => "boolean".to_string(),
                Token::Float => "float".to_string(),
                Token::Decimal => "decimal".to_string(),
                Token::Byte => "byte".to_string(),
                Token::Anydata => "anydata".to_string(),
                Token::Identifier(s) => s,
                t => return Err(self.error_previous(&format!("Expected type, found {:?}", t), Some("type"))),
            };
            TypeDescriptor::Basic(base_type)
        };

        // Handle type suffixes: arrays [], [n], [*], optional ?, union |
        loop {
            if self.match_token(&[Token::LBracket])? {
                let dimension = if self.check(&Token::RBracket) {
                    Some(ArrayDimension::Open)
                } else if self.match_token(&[Token::Star])? {
                    Some(ArrayDimension::Inferred)
                } else if let Some(Token::Number(n)) = self.peek() {
                    let num = *n as usize;
                    self.advance()?;
                    Some(ArrayDimension::Fixed(num))
                } else if matches!(self.peek(), Some(Token::Identifier(_))) {
                    // Constant reference like LENGTH
                    self.advance()?; // Skip identifier
                    None // Treat as open for now
                } else {
                    None
                };
                self.consume(Token::RBracket, "Expected ']' after array dimension", Some("']'"))?;
                type_desc = TypeDescriptor::Array {
                    element_type: Box::new(type_desc),
                    dimension,
                };
            } else if self.match_token(&[Token::Question])? {
                type_desc = TypeDescriptor::Optional(Box::new(type_desc));
            } else if self.match_token(&[Token::Pipe])? {
                let mut types = vec![type_desc];
                types.push(self.parse_type_descriptor()?);
                type_desc = TypeDescriptor::Union(types);
            } else {
                break;
            }
        }

        Ok(type_desc)
    }

    /// Utility to build a span-aware binary expression node.
    fn make_binary_expr(&self, left: Expr, op: BinaryOp, op_span: Span, right: Expr) -> Expr {
        let span_start = left.span().start.min(op_span.start);
        let span_end = right.span().end.max(op_span.end);
        Expr::Binary {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span: span_start..span_end,
        }
    }

    /// Utility to build a span-aware unary expression node.
    fn make_unary_expr(&self, op: UnaryOp, op_span: Span, operand: Expr) -> Expr {
        let span_end = operand.span().end.max(op_span.end);
        Expr::Unary {
            op,
            operand: Box::new(operand),
            span: op_span.start..span_end,
        }
    }

    /// Wraps an expression with grouping metadata for parentheses.
    fn make_grouping_expr(&self, open_span: Span, expr: Expr, close_span: Span) -> Expr {
        Expr::Grouping {
            expression: Box::new(expr),
            span: open_span.start..close_span.end,
        }
    }

    /// Constructs a literal expression with its original source span.
    fn make_literal_expr(&self, value: Literal, span: Span) -> Expr {
        Expr::Literal { value, span }
    }

    /// Builds a call expression while tracking the span of every argument.
    fn make_call_expr(
        &self,
        callee: Expr,
        arguments: Vec<Expr>,
        open_span: Span,
        close_span: Span,
    ) -> Expr {
        let callee_span = callee.span().clone();
        let mut span_start = callee_span.start.min(open_span.start);
        let mut span_end = callee_span.end.max(close_span.end);
        for argument in &arguments {
            span_start = span_start.min(argument.span().start);
            span_end = span_end.max(argument.span().end);
        }
        Expr::Call {
            callee: Box::new(callee),
            arguments,
            span: span_start..span_end,
        }
    }

    //-------------- Helpers ---------------------------

    /// Checks if the parser has reached the end of the token stream.
    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    /// Peeks at the current token without consuming it.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current).map(|(_, token, _)| token)
    }

    /// Returns the previously consumed token if one exists.
    fn previous(&self) -> Option<&Token> {
        if self.current == 0 {
            None
        } else {
            Some(&self.tokens[self.current - 1].1)
        }
    }

    /// Consumes the current token and advances the parser.
    fn advance(&mut self) -> ParseResult<&Token> {
        if self.is_at_end() {
            Err(self.unexpected_eof(None))
        } else {
            self.current += 1;
            Ok(self.previous().expect("advanced past start"))
        }
    }

    /// Consumes the current token and returns an owned clone for pattern matching.
    fn advance_owned(&mut self) -> ParseResult<Token> {
        self.advance().cloned()
    }

    /// Checks whether the current token matches the provided token kind.
    fn check(&self, expected: &Token) -> bool {
        matches!(self.peek(), Some(token) if token == expected)
    }

    /// Advances past the current token if it matches any of the provided kinds.
    fn match_token(&mut self, types: &[Token]) -> ParseResult<bool> {
        if let Some(current) = self.peek() {
            for token in types {
                if current == token {
                    let _ = self.advance()?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Consumes the expected token or returns a `ParseError` describing the mismatch.
    fn consume(
        &mut self,
        expected: Token,
        msg: &str,
        expected_lexeme: Option<&'static str>,
    ) -> ParseResult<()> {
        if self.check(&expected) {
            let _ = self.advance()?;
            Ok(())
        } else {
            Err(self.error_here(msg, expected_lexeme))
        }
    }

    /// Retrieves the span for the token at the provided index, falling back to the
    /// end-of-input span when the index is out of bounds.
    fn span_at(&self, index: usize) -> Span {
        if let Some(&(start, _, end)) = self.tokens.get(index) {
            start..end
        } else {
            self.end_span()
        }
    }

    /// Span covering the token currently under examination.
    fn current_span(&self) -> Span {
        if self.current < self.tokens.len() {
            self.span_at(self.current)
        } else {
            self.end_span()
        }
    }

    /// Span covering the token most recently consumed.
    fn previous_span(&self) -> Span {
        if self.current == 0 {
            self.end_span()
        } else {
            self.span_at(self.current - 1)
        }
    }

    /// Zero-width span at the end of the input stream.
    fn end_span(&self) -> Span {
        let end = self.tokens.last().map(|&(_, _, end)| end).unwrap_or(0);
        end..end
    }

    /// Constructs a `ParseError` for the current token position.
    fn error_here(&self, message: &str, expected: Option<&'static str>) -> ParseError {
        ParseError::new(message.to_string(), self.current_span(), expected)
    }

    /// Constructs a `ParseError` for the previously consumed token position.
    fn error_previous(&self, message: &str, expected: Option<&'static str>) -> ParseError {
        ParseError::new(message.to_string(), self.previous_span(), expected)
    }

    /// Constructs a `ParseError` representing an unexpected end of input.
    fn unexpected_eof(&self, expected: Option<&'static str>) -> ParseError {
        ParseError::new("Unexpected end of input", self.end_span(), expected)
    }

    /// Peeks ahead by `offset` tokens without consuming them.
    fn peek_n(&self, offset: usize) -> Option<&Token> {
        self.tokens
            .get(self.current + offset)
            .map(|(_, token, _)| token)
    }

    /// Determines whether the upcoming tokens form the start of a variable declaration.
    fn starts_var_decl(&self) -> bool {
        match self.peek() {
            Some(Token::Var) | Some(Token::Final) | Some(Token::Const) => true,
            Some(token) if Self::is_type_start(token) => {
                // Could be: int x, int[] x, int[3] x, etc.
                // Need to skip type suffixes to find identifier
                let mut offset = 1;
                
                // Skip array brackets and other type suffixes
                loop {
                    match self.peek_n(offset) {
                        Some(Token::LBracket) => {
                            // Skip [, maybe a number, identifier, or *, then ]
                            offset += 1;
                            if matches!(self.peek_n(offset), Some(Token::Number(_)) | Some(Token::Star) | Some(Token::Identifier(_))) {
                                offset += 1;
                            }
                            if matches!(self.peek_n(offset), Some(Token::RBracket)) {
                                offset += 1;
                            } else {
                                return false; // Malformed
                            }
                        }
                        Some(Token::Question) | Some(Token::Pipe) => {
                            offset += 1;
                        }
                        Some(Token::Lt) if matches!(token, Token::Map) => {
                            // map<T> type - skip until >
                            offset += 1;
                            // This is simplified - real impl would need to recursively parse type
                            while !matches!(self.peek_n(offset), Some(Token::Gt) | None) {
                                offset += 1;
                            }
                            if matches!(self.peek_n(offset), Some(Token::Gt)) {
                                offset += 1;
                            }
                        }
                        Some(Token::Identifier(_)) => {
                            return true;
                        }
                        _ => return false,
                    }
                }
            }
            Some(Token::Map) => {
                // map<T> identifier
                // Simplified check
                true
            }
            Some(Token::Identifier(_)) => matches!(self.peek_n(1), Some(Token::Identifier(_))),
            _ => false,
        }
    }

    /// Returns true when the token can begin a simple type descriptor in our subset.
    fn is_type_start(token: &Token) -> bool {
        matches!(
            token,
            Token::Int | Token::String | Token::Boolean | Token::Float | Token::Decimal | Token::Byte | Token::Anydata | Token::Map
        )
    }
}
