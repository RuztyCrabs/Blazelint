//! Recursive-descent parser for the Blazelint front-end.
//!
//! The parser consumes the token stream emitted by the lexer and produces a
//! span-aware abstract syntax tree. It keeps precedence handling close to the
//! grammar specification so that follow-up stages can rely on predictable AST
//! shapes and accurate byte ranges for diagnostics.
use crate::ast::*;
use crate::errors::{ParseError, Span};
use crate::lexer::Token;

/// Convenient alias for parser results carrying a `ParseError` on failure.
type ParseResult<T> = Result<T, ParseError>;

/// Stateful parser that walks the token list and builds AST nodes.
pub struct Parser {
    tokens: Vec<(usize, Token, usize)>,
    current: usize,
}

impl Parser {
    /// Creates a parser over the provided token triples produced by the lexer.
    pub fn new(tokens: Vec<(usize, Token, usize)>) -> Self {
        Self { tokens, current: 0 }
    }

    /// Parses the entire token stream into a list of top-level statements.
    ///
    /// Returning a `ParseResult` allows callers to surface rich diagnostics
    /// instead of aborting the process on the first syntax error.
    pub fn parse(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        Ok(statements)
    }

    /// Parses a top-level declaration (variable, function, or statement).
    fn declaration(&mut self) -> ParseResult<Stmt> {
        if self.starts_var_decl() || matches!(self.peek(), Some(Token::Const)) {
            self.var_decl()
        } else if matches!(self.peek(), Some(Token::Function)) {
            self.function()
        } else {
            self.statement()
        }
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
            let type_name = self.parse_type()?;
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

            (ident, name_span, Some(type_name), initializer)
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
            self.consume(Token::LBrace, "Expected '{' before else block", Some("'{'"))?;
            let else_block = self.block()?;
            span_end = self.previous_span().end;
            Some(else_block)
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
            let param_token = self.advance_owned()?;
            let param_name = match param_token {
                Token::Identifier(name) => name,
                _ => return Err(self.error_previous("Expected parameter name", Some("identifier"))),
            };
            self.consume(Token::Colon, "Expected ':' in parameter", Some("':'"))?;
            let param_type = self.parse_type()?;
            params.push((param_name, param_type));
            if !self.check(&Token::RParen) {
                self.consume(Token::Comma, "Expected ',' between parameters", Some("','"))?;
            }
        }
        self.consume(Token::RParen, "Expected ')' after parameters", Some("')'"))?;

        let return_type = if self.match_token(&[Token::Returns])? {
            Some(self.parse_type()?)
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
        let expr = self.logic_or()?;

        if self.match_token(&[Token::Eq])? {
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
                return Ok(Expr::Assign {
                    name,
                    value: Box::new(value),
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
        let mut expr = self.term()?;

        while self.match_token(&[Token::Gt, Token::Ge, Token::Lt, Token::Le])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.term()?;
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

    /// Parses an additive expression (`+`, `-`).
    fn term(&mut self) -> ParseResult<Expr> {
        let mut expr = self.factor()?;

        while self.match_token(&[Token::Plus, Token::Minus])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.factor()?;
            let op = match op_token {
                Token::Plus => BinaryOp::Plus,
                Token::Minus => BinaryOp::Minus,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses a multiplicative expression (`*`, `/`).
    fn factor(&mut self) -> ParseResult<Expr> {
        let mut expr = self.unary()?;

        while self.match_token(&[Token::Star, Token::Slash])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.unary()?;
            let op = match op_token {
                Token::Star => BinaryOp::Star,
                Token::Slash => BinaryOp::Slash,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses a unary expression (`!`, unary `-`).
    fn unary(&mut self) -> ParseResult<Expr> {
        if self.match_token(&[Token::Bang, Token::Minus])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let op = match op_token {
                Token::Bang => UnaryOp::Bang,
                Token::Minus => UnaryOp::Minus,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            return Ok(self.make_unary_expr(op, op_span, right));
        }

        self.call()
    }

    /// Parses postfix function-call chains.
    fn call(&mut self) -> ParseResult<Expr> {
        let mut expr = self.primary()?;
        loop {
            if self.match_token(&[Token::LParen])? {
                let open_span = self.previous_span();
                expr = self.finish_call(expr, open_span)?;
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
            Token::Identifier(name) => Ok(Expr::Variable {
                name,
                span: token_span,
            }),
            Token::LParen => {
                let open_span = token_span;
                let expr = self.expression()?;
                self.consume(Token::RParen, "Expected ')' after expression", Some("')'"))?;
                let close_span = self.previous_span();
                Ok(self.make_grouping_expr(open_span, expr, close_span))
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
                matches!(self.peek_n(1), Some(Token::Identifier(_)))
            }
            Some(Token::Identifier(_)) => matches!(self.peek_n(1), Some(Token::Identifier(_))),
            _ => false,
        }
    }

    /// Returns true when the token can begin a simple type descriptor in our subset.
    fn is_type_start(token: &Token) -> bool {
        matches!(
            token,
            Token::Int | Token::String | Token::Boolean | Token::Float
        )
    }
}
