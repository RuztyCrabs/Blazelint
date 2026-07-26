//! Recursive-descent parser for the Blazelint front-end.
//!
//! The parser consumes the token stream emitted by the lexer and produces a
//! span-aware abstract syntax tree. It keeps precedence handling close to the
//! grammar specification so that follow-up stages can rely on predictable AST
//! shapes and accurate byte ranges for diagnostics.
use crate::ast::*;
use crate::errors::{Diagnostic, ParseError, Span};
use crate::lexer::Token;

/// Convenient alias for parser results carrying a `ParseError` on failure.
type ParseResult<T> = Result<T, ParseError>;

/// Stateful parser that walks the token list and builds AST nodes.
pub struct Parser {
    tokens: Vec<(usize, Token, usize)>,
    current: usize,
    errors: Vec<Diagnostic>,
    /// True while parsing the type on the right of an `is` operator, where a
    /// trailing `?` may be a ternary rather than an optional-type suffix.
    in_is_type: bool,
    /// True while parsing the true-branch of a ternary, where a following `:`
    /// terminates the branch rather than forming a qualified reference.
    in_ternary_branch: bool,
}

impl Parser {
    /// Creates a parser over the provided token triples produced by the lexer.
    pub fn new(tokens: Vec<(usize, Token, usize)>) -> Self {
        Self {
            tokens,
            current: 0,
            errors: Vec::new(),
            in_is_type: false,
            in_ternary_branch: false,
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
        // Reset transient parse state on recovery.
        self.in_is_type = false;
        self.in_ternary_branch = false;
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
                | Some(Token::Const)
                | Some(Token::Type)
                | Some(Token::Enum)
                | Some(Token::Class)
                | Some(Token::Configurable)
                | Some(Token::Isolated) => return,
                _ => {
                    self.advance().ok();
                }
            }
        }
    }

    /// Parses a top-level declaration (import, type/enum/class/function/variable,
    /// or a statement). Leading module-level qualifiers (`public`, `isolated`,
    /// `readonly`, ...) are consumed first, then the declaration kind dispatched.
    fn declaration(&mut self) -> ParseResult<Stmt> {
        // Annotation attachments (`@http:ServiceConfig { ... }`) may precede any
        // declaration; consume them (parse-tolerant — not retained).
        self.skip_annotations()?;

        if self.match_token(&[Token::Import])? {
            return self.import_declaration();
        }

        // Contextual module-level declarations keyed by a leading identifier.
        // `service` is only a service declaration when not acting as a qualifier
        // (as in `service class C` / `distinct service object { }`).
        if let Some(Token::Identifier(word)) = self.peek() {
            match word.as_str() {
                "service" if !matches!(self.peek_n(1), Some(Token::Class | Token::Object)) => {
                    return self.service_declaration()
                }
                "listener" => return self.listener_declaration(),
                "annotation" => return self.annotation_declaration(),
                _ => {}
            }
        }
        if matches!(self.peek(), Some(Token::Xmlns)) {
            return self.xmlns_declaration();
        }
        if matches!(self.peek(), Some(Token::Configurable)) {
            return self.configurable_declaration();
        }

        // Destructuring variable declarations (`var {a,b} = e;`, `[a,b] = e;`,
        // `Rec {x,y} = r;`).
        if self.starts_destructure() {
            return self.destructure_decl();
        }

        // Consume any run of leading qualifiers (public/isolated/readonly/...).
        let (is_public, qualifiers) = self.parse_leading_qualifiers()?;

        match self.peek() {
            Some(Token::Type) => self.type_definition(is_public),
            Some(Token::Enum) => self.enum_definition(is_public),
            Some(Token::Class) => self.class_definition(is_public, qualifiers),
            // `function (` begins a function-typed variable declaration
            // (`function (int) returns int f = ...`); `function name(` is a decl.
            Some(Token::Function) if matches!(self.peek_n(1), Some(Token::LParen)) => {
                self.var_decl()
            }
            Some(Token::Function) => self.function(is_public, qualifiers),
            _ if self.starts_var_decl() || matches!(self.peek(), Some(Token::Const)) => {
                self.var_decl()
            }
            _ => self.statement(),
        }
    }

    /// Detects the start of a destructuring variable declaration:
    /// `var {..}`/`var [..]`, an untyped `[..] =`, or a typed `<name> {..}`.
    fn starts_destructure(&self) -> bool {
        if matches!(self.peek(), Some(Token::Var))
            && matches!(self.peek_n(1), Some(Token::LBrace | Token::LBracket))
        {
            return true;
        }
        // Untyped list `[a, b] =` or mapping `{a, b} =` destructure.
        if self.check(&Token::LBracket) {
            if let Some(end) = self.scan_balanced(0, &Token::LBracket, &Token::RBracket) {
                if matches!(self.peek_n(end), Some(Token::Eq)) {
                    return true;
                }
            }
        }
        if self.check(&Token::LBrace) {
            if let Some(end) = self.scan_balanced(0, &Token::LBrace, &Token::RBrace) {
                if matches!(self.peek_n(end), Some(Token::Eq)) {
                    return true;
                }
            }
        }
        // Typed mapping destructure `<name> {a, b} = ...`.
        if matches!(self.peek(), Some(Token::Identifier(_))) {
            if let Some(end) = self.skip_type(0) {
                if matches!(self.peek_n(end), Some(Token::LBrace)) {
                    return true;
                }
            }
        }
        false
    }

    /// Parses a destructuring variable declaration, collecting the bound names.
    fn destructure_decl(&mut self) -> ParseResult<Stmt> {
        let start = self.current_span().start;
        self.match_token(&[Token::Var])?; // optional `var`
                                          // Optional leading type (`Rec {x, y} = ...`).
        if !self.check(&Token::LBrace) && !self.check(&Token::LBracket) {
            let _ = self.parse_type_descriptor()?;
        }
        let mut names = Vec::new();
        let mut name_spans = Vec::new();
        self.parse_binding_pattern(&mut names, &mut name_spans)?;
        self.consume(
            Token::Eq,
            "Expected '=' in destructuring declaration",
            Some("'='"),
        )?;
        let initializer = self.expression()?;
        self.consume(
            Token::Semicolon,
            "Expected ';' after destructuring declaration",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::DestructureDecl {
            names,
            name_spans,
            initializer,
            span: start..end,
        })
    }

    /// Parses a list `[..]` or mapping `{..}` binding pattern, appending every
    /// bound identifier (including nested and rest bindings) to `names`.
    fn parse_binding_pattern(
        &mut self,
        names: &mut Vec<String>,
        spans: &mut Vec<Span>,
    ) -> ParseResult<()> {
        if self.match_token(&[Token::LBrace])? {
            while !self.check(&Token::RBrace) && !self.is_at_end() {
                if self.match_token(&[Token::DotDotDot])? {
                    names.push(self.expect_ident("Expected name after '...'")?);
                    spans.push(self.previous_span());
                } else if self.check(&Token::LBrace) || self.check(&Token::LBracket) {
                    self.parse_binding_pattern(names, spans)?;
                } else {
                    let key = self.expect_ident_or_string("Expected field in binding pattern")?;
                    let key_span = self.previous_span();
                    if self.match_token(&[Token::Colon])? {
                        // `key: <nested-pattern>`
                        if self.check(&Token::LBrace) || self.check(&Token::LBracket) {
                            self.parse_binding_pattern(names, spans)?;
                        } else {
                            names.push(self.expect_ident("Expected binding after ':'")?);
                            spans.push(self.previous_span());
                        }
                    } else {
                        names.push(key);
                        spans.push(key_span);
                    }
                }
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
            self.consume(
                Token::RBrace,
                "Expected '}' in binding pattern",
                Some("'}'"),
            )?;
        } else if self.match_token(&[Token::LBracket])? {
            while !self.check(&Token::RBracket) && !self.is_at_end() {
                if self.match_token(&[Token::DotDotDot])? {
                    names.push(self.expect_ident("Expected name after '...'")?);
                    spans.push(self.previous_span());
                } else if self.check(&Token::LBrace) || self.check(&Token::LBracket) {
                    self.parse_binding_pattern(names, spans)?;
                } else {
                    names.push(self.expect_ident("Expected binding in list pattern")?);
                    spans.push(self.previous_span());
                }
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
            self.consume(
                Token::RBracket,
                "Expected ']' in binding pattern",
                Some("']'"),
            )?;
        }
        Ok(())
    }

    /// Consumes any run of annotation attachments (`@tag`, `@mod:tag { ... }`),
    /// discarding them. Annotation values are mapping constructors and are skipped
    /// as balanced brace blocks.
    fn skip_annotations(&mut self) -> ParseResult<()> {
        while self.match_token(&[Token::At])? {
            let _ = self.expect_ident("Expected annotation tag after '@'")?;
            if self.match_token(&[Token::Colon])? {
                let _ = self.expect_ident("Expected annotation name after ':'")?;
            }
            if self.check(&Token::LBrace) {
                self.skip_braced_block()?;
            }
        }
        Ok(())
    }

    /// Consumes a run of module-level qualifier keywords, returning whether
    /// `public` was seen and the list of qualifier lexemes (for AST retention).
    ///
    /// Only `public` and `isolated` are treated as leading qualifiers here.
    /// `readonly`, `distinct`, and `transactional` are deliberately excluded
    /// because they are primarily type-level constructors (`readonly & T`,
    /// `distinct T`) that must reach `parse_type_descriptor` at declaration start.
    fn parse_leading_qualifiers(&mut self) -> ParseResult<(bool, Vec<String>)> {
        let mut is_public = false;
        let mut qualifiers = Vec::new();
        loop {
            if self.match_token(&[Token::Public])? {
                is_public = true;
                qualifiers.push("public".to_string());
            } else if self.match_token(&[Token::Isolated])? {
                qualifiers.push("isolated".to_string());
            } else if self.check_ctx_kw("distinct")
                && (matches!(self.peek_n(1), Some(Token::Class))
                    || matches!(self.peek_n(1), Some(Token::Identifier(s)) if matches!(s.as_str(), "service" | "client")))
            {
                // `distinct` as a class qualifier (`distinct service class C`),
                // not the `distinct T` type constructor used in type position.
                self.advance()?;
                qualifiers.push("distinct".to_string());
            } else if matches!(
                self.peek(),
                Some(Token::Identifier(s)) if matches!(s.as_str(), "transactional" | "client" | "service")
            ) && !matches!(self.peek_n(1), Some(Token::Slash))
            {
                // `service` here is a class qualifier (`distinct service class C`),
                // not a service declaration (`service /path on ...`).
                if let Token::Identifier(s) = self.advance_owned()? {
                    qualifiers.push(s);
                }
            } else {
                break;
            }
        }
        Ok((is_public, qualifiers))
    }

    /// Parses an import declaration (import ballerina/io;).
    fn import_declaration(&mut self) -> ParseResult<Stmt> {
        let import_span_start = self.previous_span().start;

        // Module path segments separated by `/` (org) and `.` (submodules), e.g.
        // `ballerina/lang.runtime` or `ballerinax/aws.lambda`.
        let mut package_path = Vec::new();
        package_path.push(self.expect_ident("Expected package name after 'import'")?);
        loop {
            if self.match_token(&[Token::Slash, Token::Dot])? {
                package_path.push(self.expect_ident("Expected package component")?);
            } else {
                break;
            }
        }

        // Optional import alias: `import foo/bar as baz;`.
        if self.check_ctx_kw("as") {
            self.advance()?; // 'as'
                             // The alias may be an identifier or `_` (no prefix).
            let _ = self.advance_owned()?;
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
            // Constants may carry an optional type annotation, e.g.
            // `const int MAX = 5;` or `const MAX = 5;`.
            let type_annotation = if self.const_has_type_annotation() {
                Some(self.parse_type_descriptor()?)
            } else {
                None
            };

            let name = self.expect_ident("Expected constant name after 'const'")?;
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
                type_annotation,
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

    /// Parses a single statement (control flow or expression).
    fn statement(&mut self) -> ParseResult<Stmt> {
        match self.peek() {
            Some(Token::If) => self.if_statement(),
            Some(Token::While) => self.while_statement(),
            Some(Token::Foreach) => self.foreach_statement(),
            // A query expression may stand alone as a statement (e.g. a
            // `from ... do { ... }` query action).
            _ if self.check_ctx_kw("from") => {
                let expr = self.expression()?;
                let start = expr.span().start;
                // A query-action statement needs no trailing `;` when it ends in a
                // `do { }` block, but one is allowed.
                self.match_token(&[Token::Semicolon])?;
                let end = self.previous_span().end;
                Ok(Stmt::Expression {
                    expression: expr,
                    span: start..end.max(start),
                })
            }
            Some(Token::Match) => self.match_statement(),
            Some(Token::Lock) => self.lock_statement(),
            Some(Token::Do) => self.do_statement(),
            // `transaction {` is a statement; `transaction:` is a qualified
            // reference into the transaction lang library.
            Some(Token::Transaction) if !matches!(self.peek_n(1), Some(Token::Colon)) => {
                self.transaction_statement()
            }
            Some(Token::Retry) => self.retry_statement(),
            Some(Token::Fork) => self.fork_statement(),
            Some(Token::Worker) => self.worker_statement(),
            Some(Token::Fail) => {
                self.advance()?;
                let start = self.previous_span().start;
                let value = self.expression()?;
                self.consume(Token::Semicolon, "Expected ';' after fail", Some("';'"))?;
                let end = self.previous_span().end;
                Ok(Stmt::Fail {
                    value,
                    span: start..end,
                })
            }
            Some(Token::Rollback) => {
                self.advance()?;
                let start = self.previous_span().start;
                if !self.check(&Token::Semicolon) {
                    let _ = self.expression()?;
                }
                self.consume(Token::Semicolon, "Expected ';' after rollback", Some("';'"))?;
                let end = self.previous_span().end;
                Ok(Stmt::Rollback { span: start..end })
            }
            // A leading `{` at statement position is a block statement, not a
            // mapping-constructor expression.
            Some(Token::LBrace) => {
                self.advance()?; // consume '{'
                let start = self.previous_span().start;
                let body = self.block()?;
                let end = self.previous_span().end;
                Ok(Stmt::Block {
                    body,
                    span: start..end,
                })
            }
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
        // Parentheses around the condition are optional in Ballerina; a leading
        // `(` is parsed as a grouping expression, so `if (x)` and `if x` both work.
        let condition = self.expression()?;

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
        // Parentheses around the condition are optional (see `if_statement`).
        let condition = self.expression()?;
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

        // Ballerina foreach binds a typed binding pattern: `foreach <type> <var>`
        // or `foreach var <var>`. Detect which form we have:
        //   - `var`            -> no explicit type annotation
        //   - `<ident> in ...` -> the identifier is the loop variable (no type)
        //   - otherwise        -> a leading type descriptor
        let is_untyped_binding = self.match_token(&[Token::Var])?
            || (matches!(self.peek(), Some(Token::Identifier(_)))
                && matches!(self.peek_n(1), Some(Token::In)));
        let type_annotation = if is_untyped_binding {
            None
        } else {
            Some(self.parse_type_descriptor()?)
        };

        // The binding is either a simple name or a destructuring pattern
        // (`foreach [string, int] [name, grade] in ...`).
        let (variable, extra_bindings) =
            if self.check(&Token::LBracket) || self.check(&Token::LBrace) {
                let mut names = Vec::new();
                let mut spans = Vec::new();
                self.parse_binding_pattern(&mut names, &mut spans)?;
                let mut iter = names.into_iter();
                let first = iter.next().unwrap_or_default();
                (first, iter.collect())
            } else {
                (
                    self.expect_ident("Expected variable name in foreach")?,
                    Vec::new(),
                )
            };

        self.consume(
            Token::In,
            "Expected 'in' after foreach variable",
            Some("'in'"),
        )?;
        let iterable = self.expression()?;
        self.consume(
            Token::LBrace,
            "Expected '{' before foreach body",
            Some("'{'"),
        )?;
        let body = self.block()?;
        let span_end = self.previous_span().end;

        Ok(Stmt::Foreach {
            type_annotation,
            variable,
            extra_bindings,
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

    /// Consumes a `{` and parses the enclosed block.
    fn braced_block(&mut self) -> ParseResult<Vec<Stmt>> {
        self.consume(Token::LBrace, "Expected '{'", Some("'{'"))?;
        self.block()
    }

    /// Parses a `match` statement.
    fn match_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'match'
        let start = self.previous_span().start;
        let subject = self.expression()?;
        self.consume(
            Token::LBrace,
            "Expected '{' after match subject",
            Some("'{'"),
        )?;
        let mut arms = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            arms.push(self.match_arm()?);
        }
        self.consume(Token::RBrace, "Expected '}' after match arms", Some("'}'"))?;
        let end = self.previous_span().end;
        Ok(Stmt::Match {
            subject,
            arms,
            span: start..end,
        })
    }

    /// Parses a single `match` arm: `pattern (| pattern)* [if guard] => { body }`.
    fn match_arm(&mut self) -> ParseResult<MatchArm> {
        let mut patterns = vec![self.match_pattern()?];
        while self.match_token(&[Token::Pipe])? {
            patterns.push(self.match_pattern()?);
        }
        let guard = if self.match_token(&[Token::If])? {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(Token::Arrow, "Expected '=>' in match arm", Some("'=>'"))?;
        let body = self.braced_block()?;
        let mut bindings = Vec::new();
        for pattern in &patterns {
            Self::collect_pattern_bindings(pattern, &mut bindings);
        }
        Ok(MatchArm {
            patterns,
            bindings,
            guard,
            body,
        })
    }

    /// Parses a single match pattern.
    fn match_pattern(&mut self) -> ParseResult<MatchPattern> {
        // `var` prefix: `var x` capture, or `var {..}` / `var [..]` destructuring.
        if self.check(&Token::Var)
            && !matches!(self.peek_n(1), Some(Token::LBrace) | Some(Token::LBracket))
        {
            self.advance()?; // 'var'
            let name = self.expect_ident("Expected binding name after 'var'")?;
            return Ok(MatchPattern::Binding(name));
        }
        self.match_token(&[Token::Var])?; // optional `var` before a destructuring
                                          // Rest pattern `...rest`.
        if self.match_token(&[Token::DotDotDot])? {
            let name = self.expect_ident("Expected name after '...'")?;
            return Ok(MatchPattern::Rest(name));
        }
        // Nil pattern `()`.
        if self.check(&Token::LParen) && matches!(self.peek_n(1), Some(Token::RParen)) {
            self.advance()?;
            self.advance()?;
            return Ok(MatchPattern::Literal("()".to_string()));
        }
        // List pattern `[p, ...]`.
        if self.match_token(&[Token::LBracket])? {
            let mut items = Vec::new();
            if !self.check(&Token::RBracket) {
                loop {
                    items.push(self.match_pattern()?);
                    if !self.match_token(&[Token::Comma])? {
                        break;
                    }
                }
            }
            self.consume(Token::RBracket, "Expected ']' in list pattern", Some("']'"))?;
            return Ok(MatchPattern::List(items));
        }
        // Mapping pattern `{ key: p, ... }`.
        if self.match_token(&[Token::LBrace])? {
            let mut fields = Vec::new();
            if !self.check(&Token::RBrace) {
                loop {
                    // Rest field `...rest`.
                    if self.match_token(&[Token::DotDotDot])? {
                        let name = self.expect_ident("Expected name after '...'")?;
                        fields.push((String::new(), MatchPattern::Rest(name)));
                        self.match_token(&[Token::Comma])?;
                        break;
                    }
                    let key = self.expect_ident_or_string("Expected key in mapping pattern")?;
                    // `{ key: pattern }` or shorthand `{ key }` (== `{ key: key }`).
                    let value = if self.match_token(&[Token::Colon])? {
                        self.match_pattern()?
                    } else {
                        MatchPattern::Binding(key.clone())
                    };
                    fields.push((key, value));
                    if !self.match_token(&[Token::Comma])? {
                        break;
                    }
                }
            }
            self.consume(
                Token::RBrace,
                "Expected '}' in mapping pattern",
                Some("'}'"),
            )?;
            return Ok(MatchPattern::Mapping(fields));
        }
        // Literal patterns.
        if matches!(
            self.peek(),
            Some(Token::Number(_))
                | Some(Token::StringLiteral(_))
                | Some(Token::True)
                | Some(Token::False)
                | Some(Token::Minus)
        ) {
            let mut text = String::new();
            if self.match_token(&[Token::Minus])? {
                text.push('-');
            }
            let token = self.advance_owned()?;
            let rendered = match token {
                Token::Number(n) => n.to_string(),
                Token::StringLiteral(s) => format!("\"{s}\""),
                Token::True => "true".to_string(),
                Token::False => "false".to_string(),
                _ => unreachable!(),
            };
            text.push_str(&rendered);
            return Ok(MatchPattern::Literal(text));
        }
        // Identifier: wildcard `_`, `error(...)` pattern, or a binding/const ref.
        let name = self.expect_ident("Expected match pattern")?;
        if name == "_" {
            return Ok(MatchPattern::Wildcard);
        }
        if name == "error" && self.check(&Token::LParen) {
            self.advance()?; // '('
            let mut items = Vec::new();
            if !self.check(&Token::RParen) {
                loop {
                    items.push(self.match_pattern()?);
                    if !self.match_token(&[Token::Comma])? {
                        break;
                    }
                }
            }
            self.consume(Token::RParen, "Expected ')' in error pattern", Some("')'"))?;
            return Ok(MatchPattern::Error(items));
        }
        Ok(MatchPattern::Binding(name))
    }

    /// Collects the variable names bound by a pattern (captures and rests).
    fn collect_pattern_bindings(pattern: &MatchPattern, out: &mut Vec<String>) {
        match pattern {
            MatchPattern::Binding(name) | MatchPattern::Rest(name) => out.push(name.clone()),
            MatchPattern::List(items) | MatchPattern::Error(items) => {
                for item in items {
                    Self::collect_pattern_bindings(item, out);
                }
            }
            MatchPattern::Mapping(fields) => {
                for (_key, value) in fields {
                    Self::collect_pattern_bindings(value, out);
                }
            }
            MatchPattern::Wildcard | MatchPattern::Literal(_) => {}
        }
    }

    /// Parses a `lock { ... }` statement.
    fn lock_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'lock'
        let start = self.previous_span().start;
        let body = self.braced_block()?;
        let end = self.previous_span().end;
        Ok(Stmt::Lock {
            body,
            span: start..end,
        })
    }

    /// Parses a `do { ... } [on fail [type] <var> { ... }]` statement.
    fn do_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'do'
        let start = self.previous_span().start;
        let body = self.braced_block()?;
        let (on_fail_var, on_fail_body) = self.parse_on_fail()?;
        let end = self.previous_span().end;
        Ok(Stmt::DoOnFail {
            body,
            on_fail_var,
            on_fail_body,
            span: start..end,
        })
    }

    /// Parses an optional trailing `on fail [type] <var> { ... }` clause.
    fn parse_on_fail(&mut self) -> ParseResult<(Option<String>, Vec<Stmt>)> {
        if !self.match_token(&[Token::On])? {
            return Ok((None, Vec::new()));
        }
        self.consume(Token::Fail, "Expected 'fail' after 'on'", Some("'fail'"))?;
        // Optional error type descriptor precedes the binding variable.
        if self.match_token(&[Token::Var])? {
            // `on fail var e`
        } else if matches!(self.peek(), Some(Token::Identifier(_)))
            && !matches!(self.peek_n(1), Some(Token::LBrace))
        {
            let _ty = self.parse_type_descriptor()?;
        }
        let var = self.expect_ident("Expected error binding variable in 'on fail'")?;
        let body = self.braced_block()?;
        Ok((Some(var), body))
    }

    /// Parses a `transaction { ... }` statement.
    fn transaction_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'transaction'
        let start = self.previous_span().start;
        let body = self.braced_block()?;
        let end = self.previous_span().end;
        Ok(Stmt::Transaction {
            body,
            span: start..end,
        })
    }

    /// Parses a `retry [<...>] [(args)] [transaction] { ... }` statement.
    fn retry_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'retry'
        let start = self.previous_span().start;
        // Optional type argument `<...>`.
        if self.check(&Token::Lt) {
            if let Some(end) = self.scan_angle(0) {
                for _ in 0..end {
                    self.advance()?;
                }
            }
        }
        // Optional retry-count argument `(expr)`.
        if self.match_token(&[Token::LParen])? {
            if !self.check(&Token::RParen) {
                let _ = self.expression()?;
            }
            self.consume(Token::RParen, "Expected ')' after retry count", Some("')'"))?;
        }
        // Optional `transaction` keyword (retry-transaction statement).
        self.match_token(&[Token::Transaction])?;
        let body = self.braced_block()?;
        let end = self.previous_span().end;
        Ok(Stmt::Retry {
            body,
            span: start..end,
        })
    }

    /// Parses a `fork { ... }` statement, skipping the body leniently.
    fn fork_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'fork'
        let start = self.previous_span().start;
        self.skip_braced_block()?;
        let end = self.previous_span().end;
        Ok(Stmt::Fork { span: start..end })
    }

    /// Parses a named `worker <name> [returns T] { ... }` declaration.
    fn worker_statement(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // 'worker'
        let start = self.previous_span().start;
        let name = self.expect_ident("Expected worker name")?;
        if self.match_token(&[Token::Returns])? {
            let _ = self.parse_type_descriptor()?;
        }
        let body = self.braced_block()?;
        let end = self.previous_span().end;
        Ok(Stmt::Worker {
            name,
            body,
            span: start..end,
        })
    }

    /// Parses a `function` declaration including parameters, optional return type,
    /// and body. Leading qualifiers (`public`/`isolated`/...) are consumed by the
    /// caller and passed in.
    fn function(&mut self, is_public: bool, _qualifiers: Vec<String>) -> ParseResult<Stmt> {
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
        let params = self.parse_params()?;
        self.consume(Token::RParen, "Expected ')' after parameters", Some("')'"))?;

        let return_type = if self.match_token(&[Token::Returns])? {
            Some(self.parse_type_descriptor()?)
        } else {
            None
        };

        let body = self.named_function_body()?;
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

    /// Parses a named function/method body: either a `{ block }` or an
    /// expression body `=> expr;`.
    fn named_function_body(&mut self) -> ParseResult<Vec<Stmt>> {
        if self.match_token(&[Token::Arrow])? {
            let expr = self.expression()?;
            let span = expr.span().clone();
            self.consume(
                Token::Semicolon,
                "Expected ';' after expression-bodied function",
                Some("';'"),
            )?;
            Ok(vec![Stmt::Return {
                value: Some(expr),
                span,
            }])
        } else {
            self.consume(
                Token::LBrace,
                "Expected '{' before function body",
                Some("'{'"),
            )?;
            self.block()
        }
    }

    /// Parses a comma-separated parameter list (without the surrounding
    /// parentheses). Handles rest params (`T... name`) and default values
    /// (`T name = expr`), retaining just the name and type.
    fn parse_params(&mut self) -> ParseResult<Vec<(String, TypeDescriptor)>> {
        let mut params = Vec::new();
        while !self.check(&Token::RParen) {
            self.skip_annotations()?; // e.g. `@http:Payload T body`
            self.match_token(&[Token::Star])?; // included-record parameter `*T name`
            let param_type = self.parse_type_descriptor()?;
            self.match_token(&[Token::DotDotDot])?; // optional rest marker
            let param_name = self.expect_ident("Expected parameter name")?;
            if self.match_token(&[Token::Eq])? {
                let _default = self.expression()?;
            }
            params.push((param_name, param_type));
            if !self.check(&Token::RParen) {
                self.consume(Token::Comma, "Expected ',' between parameters", Some("','"))?;
            }
        }
        Ok(params)
    }

    /// Parses a module-level type definition: `type Name <descriptor>;`.
    fn type_definition(&mut self, is_public: bool) -> ParseResult<Stmt> {
        self.advance()?; // consume 'type'
        let keyword_span = self.previous_span();
        let name = self.expect_ident("Expected type name after 'type'")?;
        let name_span = self.previous_span();
        let descriptor = self.parse_type_descriptor()?;
        self.consume(
            Token::Semicolon,
            "Expected ';' after type definition",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::TypeDef {
            is_public,
            name,
            name_span,
            descriptor,
            span: keyword_span.start..end,
        })
    }

    /// Parses an enum definition: `enum Name { A, B = expr, ... }`.
    fn enum_definition(&mut self, is_public: bool) -> ParseResult<Stmt> {
        self.advance()?; // consume 'enum'
        let keyword_span = self.previous_span();
        let name = self.expect_ident("Expected enum name after 'enum'")?;
        let name_span = self.previous_span();
        self.consume(Token::LBrace, "Expected '{' after enum name", Some("'{'"))?;
        let mut members = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            let member_name = self.expect_ident("Expected enum member name")?;
            let member_span = self.previous_span();
            let value = if self.match_token(&[Token::Eq])? {
                Some(self.expression()?)
            } else {
                None
            };
            members.push(EnumMember {
                name: member_name,
                name_span: member_span,
                value,
            });
            if !self.match_token(&[Token::Comma])? {
                break;
            }
        }
        self.consume(
            Token::RBrace,
            "Expected '}' after enum members",
            Some("'}'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::EnumDef {
            is_public,
            name,
            name_span,
            members,
            span: keyword_span.start..end,
        })
    }

    /// Parses a class definition: `class Name { fields and methods }`. Members are
    /// parsed into real statements (fields as `VarDecl`, methods as `Function`) so
    /// downstream stages can reach them later.
    fn class_definition(&mut self, is_public: bool, qualifiers: Vec<String>) -> ParseResult<Stmt> {
        self.advance()?; // consume 'class'
        let keyword_span = self.previous_span();
        let name = self.expect_ident("Expected class name after 'class'")?;
        let name_span = self.previous_span();
        self.consume(Token::LBrace, "Expected '{' after class name", Some("'{'"))?;
        let mut members = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if let Some(member) = self.class_member()? {
                members.push(member);
            }
        }
        self.consume(Token::RBrace, "Expected '}' at end of class", Some("'}'"))?;
        let end = self.previous_span().end;
        Ok(Stmt::ClassDef {
            is_public,
            name,
            name_span,
            qualifiers,
            members,
            span: keyword_span.start..end,
        })
    }

    /// Parses a single class member. Returns `None` for members that are not
    /// modelled as statements (e.g. type inclusions `*T;`), which the caller
    /// skips.
    fn class_member(&mut self) -> ParseResult<Option<Stmt>> {
        // Annotations may precede a class/service member.
        self.skip_annotations()?;
        // Skip member qualifiers (public/private/final/isolated/remote/resource/...).
        let mut is_resource = false;
        loop {
            if self.match_token(&[Token::Public, Token::Final, Token::Isolated])? {
                continue;
            }
            // `readonly` as a member qualifier, but not `readonly & T` (a type).
            if self.check_ctx_kw("readonly") && !matches!(self.peek_n(1), Some(Token::Amp)) {
                self.advance()?;
                continue;
            }
            if self.check_ctx_kw("resource") {
                is_resource = true;
                self.advance()?;
                continue;
            }
            if matches!(
                self.peek(),
                Some(Token::Identifier(s)) if matches!(s.as_str(), "private" | "remote" | "transactional")
            ) {
                self.advance()?;
                continue;
            }
            break;
        }

        // Type inclusion `*T;` — consumed but not retained.
        if self.match_token(&[Token::Star])? {
            let _included = self.parse_type_descriptor()?;
            self.consume(
                Token::Semicolon,
                "Expected ';' after class type inclusion",
                Some("';'"),
            )?;
            return Ok(None);
        }

        // Method: `function name(params) [returns T] { body }`. Resource methods
        // are `resource function <accessor> <resource-path>(params) ...`.
        if self.check(&Token::Function) {
            self.advance()?; // 'function'
            let keyword_span = self.previous_span();
            let name = if is_resource {
                let accessor = self.expect_ident("Expected resource accessor")?;
                self.skip_resource_path_signature()?;
                accessor
            } else {
                self.expect_ident("Expected method name")?
            };
            let name_span = self.previous_span();
            self.consume(Token::LParen, "Expected '(' after method name", Some("'('"))?;
            let params = self.parse_params()?;
            self.consume(Token::RParen, "Expected ')' after parameters", Some("')'"))?;
            let return_type = if self.match_token(&[Token::Returns])? {
                Some(self.parse_type_descriptor()?)
            } else {
                None
            };
            // Methods may be block-bodied or (rarely) abstract signatures ending
            // in `;`; also support expression bodies `=> expr;`.
            let body = if self.match_token(&[Token::Semicolon])? {
                Vec::new()
            } else {
                self.named_function_body()?
            };
            let end = self.previous_span().end;
            return Ok(Some(Stmt::Function {
                is_public: false,
                name,
                name_span,
                params,
                return_type,
                body,
                span: keyword_span.start..end,
            }));
        }

        // Field: `Type name [= default];`.
        let start = self.current_span().start;
        let field_type = self.parse_type_descriptor()?;
        let name = self.expect_ident("Expected field name in class")?;
        let name_span = self.previous_span();
        let initializer = if self.match_token(&[Token::Eq])? {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(
            Token::Semicolon,
            "Expected ';' after class field",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Some(Stmt::VarDecl {
            is_final: false,
            name,
            name_span,
            type_annotation: Some(field_type),
            initializer,
            span: start..end,
        }))
    }

    /// Parses a `configurable` module variable: `configurable T name = expr | ?;`.
    fn configurable_declaration(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'configurable'
        let start = self.previous_span().start;
        let type_annotation = self.parse_type_descriptor()?;
        let name = self.expect_ident("Expected configurable variable name")?;
        let name_span = self.previous_span();
        self.consume(
            Token::Eq,
            "Configurable variables require '= <value>' or '= ?'",
            Some("'='"),
        )?;
        // `= ?` marks a required configurable whose value is supplied externally at
        // runtime. Model it as an initialized value of the declared type (via a
        // synthetic cast) so it is neither reported as uninitialized nor as a
        // type mismatch. A concrete default expression is used as-is.
        let initializer = if self.match_token(&[Token::Question])? {
            let q_span = self.previous_span();
            Some(Expr::Cast {
                type_desc: type_annotation.clone(),
                expr: Box::new(Expr::Literal {
                    value: Literal::Nil,
                    span: q_span.clone(),
                }),
                span: q_span,
            })
        } else {
            Some(self.expression()?)
        };
        self.consume(
            Token::Semicolon,
            "Expected ';' after configurable declaration",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::VarDecl {
            is_final: false,
            name,
            name_span,
            type_annotation: Some(type_annotation),
            initializer,
            span: start..end,
        })
    }

    /// Parses a `listener` declaration leniently as `listener Type name = expr;`.
    fn listener_declaration(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'listener'
        let start = self.previous_span().start;
        let _type = self.parse_type_descriptor()?;
        let name = self.expect_ident("Expected listener name")?;
        let name_span = self.previous_span();
        self.consume(
            Token::Eq,
            "Expected '=' in listener declaration",
            Some("'='"),
        )?;
        let _init = self.expression()?;
        self.consume(
            Token::Semicolon,
            "Expected ';' after listener declaration",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::ListenerDecl {
            name,
            name_span,
            span: start..end,
        })
    }

    /// Parses a `service` declaration. The header (path, `on`, listener
    /// expression) is consumed leniently up to the body-opening brace, then the
    /// body is parsed into member statements (fields and resource/remote methods).
    fn service_declaration(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'service'
        let start = self.previous_span().start;
        // Consume the header up to the top-level `{`, tracking `()`/`[]` depth so a
        // `{` inside the listener expression is not mistaken for the body.
        let mut paren_depth = 0i32;
        loop {
            match self.peek() {
                Some(Token::LParen) | Some(Token::LBracket) => paren_depth += 1,
                Some(Token::RParen) | Some(Token::RBracket) => paren_depth -= 1,
                Some(Token::LBrace) if paren_depth <= 0 => break,
                None => break,
                _ => {}
            }
            self.advance()?;
        }
        let members = self.braced_block_of_members()?;
        let end = self.previous_span().end;
        Ok(Stmt::ServiceDecl {
            members,
            span: start..end,
        })
    }

    /// Parses a `{ ... }` block of class/service members.
    fn braced_block_of_members(&mut self) -> ParseResult<Vec<Stmt>> {
        self.consume(Token::LBrace, "Expected '{'", Some("'{'"))?;
        let mut members = Vec::new();
        while !self.check(&Token::RBrace) && !self.is_at_end() {
            if let Some(member) = self.class_member()? {
                members.push(member);
            }
        }
        self.consume(Token::RBrace, "Expected '}' at end of body", Some("'}'"))?;
        Ok(members)
    }

    /// Consumes a resource method's path signature (segments after the accessor,
    /// up to the parameter list `(`), including computed `[type name]` segments.
    fn skip_resource_path_signature(&mut self) -> ParseResult<()> {
        loop {
            match self.peek() {
                Some(Token::Dot) | Some(Token::Slash) | Some(Token::Identifier(_)) => {
                    self.advance()?;
                }
                Some(Token::LBracket) => {
                    self.advance()?;
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance_owned()? {
                            Token::LBracket => depth += 1,
                            Token::RBracket => depth -= 1,
                            _ => {}
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    /// Parses an `annotation` declaration leniently up to its terminating `;`.
    fn annotation_declaration(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'annotation'
        let start = self.previous_span().start;
        while !self.check(&Token::Semicolon) && !self.is_at_end() {
            self.advance()?;
        }
        self.consume(
            Token::Semicolon,
            "Expected ';' after annotation declaration",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::AnnotationDecl { span: start..end })
    }

    /// Parses an `xmlns` namespace declaration leniently up to its `;`.
    fn xmlns_declaration(&mut self) -> ParseResult<Stmt> {
        self.advance()?; // consume 'xmlns'
        let start = self.previous_span().start;
        while !self.check(&Token::Semicolon) && !self.is_at_end() {
            self.advance()?;
        }
        self.consume(
            Token::Semicolon,
            "Expected ';' after xmlns declaration",
            Some("';'"),
        )?;
        let end = self.previous_span().end;
        Ok(Stmt::Xmlns { span: start..end })
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

            // Field/index lvalues (`self.count = x`, `arr[i] += 1`) are valid
            // assignment targets. The compound-operator distinction is not retained
            // here (member-assignment semantics are deferred).
            if matches!(expr, Expr::FieldAccess { .. } | Expr::MemberAccess { .. }) {
                let span_start = expr.span().start.min(assign_span.start);
                let span_end = value_span_end.max(assign_span.end);
                return Ok(Expr::MemberAssign {
                    target: Box::new(expr),
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

    /// Parses ternary and elvis operators (`? :`, `?:`).
    fn ternary(&mut self) -> ParseResult<Expr> {
        let mut expr = self.range_expr()?;

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
            let was_in_branch = self.in_ternary_branch;
            self.in_ternary_branch = true;
            let true_expr = self.expression()?;
            self.in_ternary_branch = was_in_branch;
            self.consume(
                Token::Colon,
                "Expected ':' in ternary expression",
                Some("':'"),
            )?;
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

    /// Parses a range expression (`a...b` inclusive or `a..<b` half-open).
    fn range_expr(&mut self) -> ParseResult<Expr> {
        let start_expr = self.logic_or()?;
        if self.match_token(&[Token::DotDotDot, Token::DotDotLt])? {
            let end_expr = self.logic_or()?;
            let span = start_expr.span().start..end_expr.span().end;
            Ok(Expr::Range {
                start: Box::new(start_expr),
                end: Box::new(end_expr),
                span,
            })
        } else {
            Ok(start_expr)
        }
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

        while self.match_token(&[Token::EqEq, Token::BangEq, Token::EqEqEq, Token::BangEqEq])? {
            let op_token = self.previous().cloned().expect("operator token");
            let op_span = self.previous_span();
            let right = self.comparison()?;
            let op = match op_token {
                Token::EqEq => BinaryOp::EqualEqual,
                Token::BangEq => BinaryOp::NotEqual,
                Token::EqEqEq => BinaryOp::EqualEqualEqual,
                Token::BangEqEq => BinaryOp::NotEqualEqual,
                _ => unreachable!(),
            };
            expr = self.make_binary_expr(expr, op, op_span, right);
        }

        Ok(expr)
    }

    /// Parses ordered comparisons (`>`, `>=`, `<`, `<=`) and the `is` type test.
    fn comparison(&mut self) -> ParseResult<Expr> {
        let mut expr = self.shift()?;

        loop {
            // `e is T` type-test: the right-hand side is a type descriptor.
            if self.match_token(&[Token::Is])? {
                let start = expr.span().start;
                let was_in_is = self.in_is_type;
                self.in_is_type = true;
                let ty = self.parse_type_descriptor()?;
                self.in_is_type = was_in_is;
                let end = self.previous_span().end;
                expr = Expr::TypeTest {
                    expr: Box::new(expr),
                    ty,
                    span: start..end,
                };
                continue;
            }
            if self.match_token(&[Token::Gt, Token::Ge, Token::Lt, Token::Le])? {
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
                continue;
            }
            break;
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

    /// Parses a unary expression (`!`, unary `-`, `+`, `~`), plus the check/error
    /// and `typeof`/`let` prefix expressions.
    fn unary(&mut self) -> ParseResult<Expr> {
        // Worker receive action `<- worker`.
        if self.match_token(&[Token::LeftArrow])? {
            let start = self.previous_span().start;
            let operand = self.unary()?;
            let end = operand.span().end;
            return Ok(Expr::Check {
                keyword: "<-".to_string(),
                expr: Box::new(operand),
                span: start..end,
            });
        }
        // check / checkpanic / trap / wait prefixes.
        if matches!(
            self.peek(),
            Some(Token::Check) | Some(Token::Checkpanic) | Some(Token::Trap) | Some(Token::Wait)
        ) {
            let keyword = match self.advance_owned()? {
                Token::Check => "check",
                Token::Checkpanic => "checkpanic",
                Token::Trap => "trap",
                Token::Wait => "wait",
                _ => unreachable!(),
            }
            .to_string();
            let start = self.previous_span().start;
            let operand = self.unary()?;
            let end = operand.span().end;
            return Ok(Expr::Check {
                keyword,
                expr: Box::new(operand),
                span: start..end,
            });
        }
        // typeof prefix.
        if self.match_token(&[Token::Typeof])? {
            let start = self.previous_span().start;
            let operand = self.unary()?;
            let end = operand.span().end;
            return Ok(Expr::TypeOf {
                expr: Box::new(operand),
                span: start..end,
            });
        }
        // let expression.
        if self.check(&Token::Let) {
            return self.let_expression();
        }

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
            } else if self.check(&Token::Dot)
                || (self.check(&Token::Question) && matches!(self.peek_n(1), Some(Token::Dot)))
            {
                // `.member`, optional-chaining `?.member`, or annotation access
                // `.@annot`.
                self.match_token(&[Token::Question])?; // optional `?` of `?.`
                self.consume(Token::Dot, "Expected '.'", Some("'.'"))?;
                let method_name = if self.match_token(&[Token::At])? {
                    self.expect_ident("Expected annotation name after '.@'")?
                } else if self.match_token(&[Token::Lt])? {
                    // XML step expression `x.<name>` / `x.<ns:name>`.
                    let mut name = self.expect_ident("Expected element name in '.<...>'")?;
                    if self.match_token(&[Token::Colon])? {
                        let local = self.expect_ident("Expected local name after ':'")?;
                        name = format!("{name}:{local}");
                    }
                    self.consume_gt("Expected '>' after XML step name")?;
                    name
                } else {
                    self.expect_ident("Expected method or field name after '.'")?
                };

                if self.match_token(&[Token::LParen])? {
                    // Method call: obj.method()
                    let mut arguments = Vec::new();
                    if !self.check(&Token::RParen) {
                        loop {
                            self.match_token(&[Token::DotDotDot])?; // spread argument
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
                    // Field access: obj.field
                    let span = expr.span().start..self.previous_span().end;
                    expr = Expr::FieldAccess {
                        object: Box::new(expr),
                        field: method_name,
                        span,
                    };
                }
            } else if self.match_token(&[Token::RightArrow])? {
                // Remote method call `client->method(args)` or client resource
                // access `client->/path/segments[.accessor](args)`.
                let method = if self.check(&Token::Slash) {
                    // Resource access: consume the `/path[/seg]...` and optional
                    // trailing `.accessor`; arguments (if any) follow.
                    self.parse_resource_path()?
                } else {
                    self.expect_ident("Expected remote method name after '->'")?
                };
                let mut arguments = Vec::new();
                if self.match_token(&[Token::LParen])? {
                    if !self.check(&Token::RParen) {
                        loop {
                            self.match_token(&[Token::DotDotDot])?; // spread argument
                            arguments.push(self.expression()?);
                            if !self.match_token(&[Token::Comma])? {
                                break;
                            }
                        }
                    }
                    self.consume(Token::RParen, "Expected ')' after arguments", Some("')'"))?;
                }
                let close_span = self.previous_span();
                let span = expr.span().start..close_span.end;
                expr = Expr::RemoteCall {
                    object: Box::new(expr),
                    method,
                    arguments,
                    span,
                };
            } else if self.match_token(&[Token::LBracket])? {
                // Member access `obj[index]`; multi-key access `t["a", "b"]` keeps
                // the first key as the representative member.
                let index = self.expression()?;
                while self.match_token(&[Token::Comma])? {
                    let _ = self.expression()?;
                }
                self.consume(Token::RBracket, "Expected ']' after index", Some("']'"))?;
                let close_span = self.previous_span();
                let span = expr.span().start..close_span.end;
                expr = Expr::MemberAccess {
                    object: Box::new(expr),
                    member: Box::new(index),
                    span,
                };
            } else if self.check(&Token::Colon) && !self.in_ternary_branch {
                // Check if this is a qualified call: module:function(...)
                // Only parse as qualified call if we have identifier:identifier pattern
                if let Expr::Variable { .. } = expr {
                    if matches!(self.peek_n(1), Some(Token::Identifier(_))) {
                        self.advance()?; // consume colon

                        // Qualified call: module:function()
                        let func_token = self.advance_owned()?;
                        let func_name = match func_token {
                            Token::Identifier(name) => name,
                            _ => {
                                return Err(self.error_previous(
                                    "Expected function name after ':'",
                                    Some("identifier"),
                                ))
                            }
                        };

                        // Build qualified name: module:function
                        let (module_name, span_start) = match &expr {
                            Expr::Variable { name, span } => (name.clone(), span.start),
                            _ => {
                                return Err(self.error_previous(
                                    "Qualified calls require module name before ':'",
                                    None,
                                ))
                            }
                        };
                        let qualified_name = format!("{}:{}", module_name, func_name);

                        if self.match_token(&[Token::LParen])? {
                            let mut arguments = Vec::new();
                            if !self.check(&Token::RParen) {
                                loop {
                                    self.match_token(&[Token::DotDotDot])?; // spread argument
                                    arguments.push(self.expression()?);
                                    if !self.match_token(&[Token::Comma])? {
                                        break;
                                    }
                                }
                            }
                            self.consume(
                                Token::RParen,
                                "Expected ')' after arguments",
                                Some("')'"),
                            )?;
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
        // Inside parentheses a `:` is unambiguous, so qualified references are
        // allowed again even within a ternary branch.
        let was_in_branch = self.in_ternary_branch;
        self.in_ternary_branch = false;
        if !self.check(&Token::RParen) {
            loop {
                self.match_token(&[Token::DotDotDot])?; // spread argument `...expr`
                arguments.push(self.expression()?);
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
        }
        self.consume(Token::RParen, "Expected ')' after arguments", Some("')'"))?;
        self.in_ternary_branch = was_in_branch;
        let close_span = self.previous_span();
        Ok(self.make_call_expr(callee, arguments, open_span, close_span))
    }

    /// Parses a client resource-access path following `->`, e.g. `/tasks`,
    /// `/tasks/[id]`, or `/tasks.post`. Returns the resource accessor method name
    /// (defaulting to `get`). Path segments are consumed but not retained.
    fn parse_resource_path(&mut self) -> ParseResult<String> {
        while self.match_token(&[Token::Slash])? {
            if matches!(self.peek(), Some(Token::Identifier(_))) {
                self.advance()?; // path segment
            } else if self.match_token(&[Token::LBracket])? {
                // Computed segment `[expr]`.
                let _ = self.expression()?;
                self.consume(
                    Token::RBracket,
                    "Expected ']' in resource path",
                    Some("']'"),
                )?;
            } else {
                break;
            }
        }
        // Optional `.accessor` (get/post/put/...); defaults to `get`.
        if self.match_token(&[Token::Dot])? {
            self.expect_ident("Expected resource accessor after '.'")
        } else {
            Ok("get".to_string())
        }
    }

    /// Parses a single-parameter arrow function `x => body`.
    fn arrow_function_single(&mut self) -> ParseResult<Expr> {
        let name = self.expect_ident("Expected arrow parameter")?;
        let start = self.previous_span().start;
        self.consume(Token::Arrow, "Expected '=>'", Some("'=>'"))?;
        let body = self.expression()?;
        let end = body.span().end;
        Ok(Expr::Arrow {
            params: vec![name],
            body: Box::new(body),
            span: start..end,
        })
    }

    /// Parses a parenthesised arrow function `(a, b) => body` (parameters may be
    /// typed: `(int a, string b) => ...`).
    fn arrow_function_parenthesised(&mut self) -> ParseResult<Expr> {
        let start = self.current_span().start;
        self.consume(Token::LParen, "Expected '('", Some("'('"))?;
        let mut params = Vec::new();
        while !self.check(&Token::RParen) {
            // Optional parameter type precedes the name.
            let has_type = self.peek().is_some_and(Self::is_type_start)
                || matches!(
                    (self.peek(), self.peek_n(1)),
                    (Some(Token::Identifier(_)), Some(Token::Identifier(_)))
                );
            if has_type {
                let _ = self.parse_type_descriptor()?;
            }
            params.push(self.expect_ident("Expected arrow parameter name")?);
            if !self.match_token(&[Token::Comma])? {
                break;
            }
        }
        self.consume(
            Token::RParen,
            "Expected ')' after arrow parameters",
            Some("')'"),
        )?;
        self.consume(
            Token::Arrow,
            "Expected '=>' in arrow function",
            Some("'=>'"),
        )?;
        let body = self.expression()?;
        let end = body.span().end;
        Ok(Expr::Arrow {
            params,
            body: Box::new(body),
            span: start..end,
        })
    }

    /// Parses an anonymous function `function (params) [returns T] { body }`.
    fn anonymous_function(&mut self) -> ParseResult<Expr> {
        self.consume(Token::Function, "Expected 'function'", Some("'function'"))?;
        let start = self.previous_span().start;
        self.consume(Token::LParen, "Expected '(' after 'function'", Some("'('"))?;
        let params = self.parse_params()?;
        self.consume(Token::RParen, "Expected ')' after parameters", Some("')'"))?;
        let return_type = if self.match_token(&[Token::Returns])? {
            Some(self.parse_type_descriptor()?)
        } else {
            None
        };
        // Expression-bodied form: `function (...) returns T => expr`.
        let body = if self.match_token(&[Token::Arrow])? {
            let expr = self.expression()?;
            let span = expr.span().clone();
            vec![Stmt::Return {
                value: Some(expr),
                span,
            }]
        } else {
            self.consume(
                Token::LBrace,
                "Expected '{' before function body",
                Some("'{'"),
            )?;
            self.block()?
        };
        let end = self.previous_span().end;
        Ok(Expr::AnonFunction {
            params,
            return_type,
            body,
            span: start..end,
        })
    }

    /// Parses a `let` expression: `let [final] T name = expr, ... in body`.
    fn let_expression(&mut self) -> ParseResult<Expr> {
        self.consume(Token::Let, "Expected 'let'", Some("'let'"))?;
        let start = self.previous_span().start;
        let mut bindings = Vec::new();
        loop {
            self.match_token(&[Token::Final])?; // optional 'final'
            let _ty = self.parse_type_descriptor()?;
            let name = self.expect_ident("Expected variable name in let binding")?;
            self.consume(Token::Eq, "Expected '=' in let binding", Some("'='"))?;
            let value = self.expression()?;
            bindings.push(LetBinding { name, value });
            if !self.match_token(&[Token::Comma])? {
                break;
            }
        }
        self.consume(Token::In, "Expected 'in' after let bindings", Some("'in'"))?;
        let body = self.expression()?;
        let end = body.span().end;
        Ok(Expr::Let {
            bindings,
            body: Box::new(body),
            span: start..end,
        })
    }

    /// Parses a query expression: `from ... <clauses> select ... [on conflict e]`.
    fn query_expression(&mut self) -> ParseResult<Expr> {
        let start = self.current_span().start;
        let mut clauses = vec![self.query_from_clause()?];
        loop {
            if self.check_ctx_kw("from") {
                clauses.push(self.query_from_clause()?);
            } else if self.match_ctx_kw("where")? {
                clauses.push(QueryClause::Where(self.expression()?));
            } else if self.check(&Token::Let) {
                clauses.push(self.query_let_clause()?);
            } else if self.check_ctx_kw("join") || self.check_ctx_kw("outer") {
                clauses.push(self.query_join_clause()?);
            } else if self.check_ctx_kw("order") {
                clauses.push(self.query_order_by_clause()?);
            } else if self.match_ctx_kw("limit")? {
                clauses.push(QueryClause::Limit(self.expression()?));
            } else if self.match_ctx_kw("group")? {
                // group by <expr> — retained without detail.
                self.match_ctx_kw("by")?;
                let _ = self.expression()?;
                clauses.push(QueryClause::Other);
            } else if self.match_ctx_kw("select")? {
                clauses.push(QueryClause::Select(self.expression()?));
                break;
            } else if self.match_ctx_kw("collect")? {
                // `collect <expr>` is a terminal clause (like select).
                clauses.push(QueryClause::Select(self.expression()?));
                break;
            } else if self.check(&Token::Do) {
                // `do { ... }` query action clause (terminal).
                self.advance()?; // 'do'
                let body = self.braced_block()?;
                clauses.push(QueryClause::Do(body));
                break;
            } else {
                break;
            }
        }
        // Optional `on conflict <expr>`.
        if self.check(&Token::On)
            && matches!(self.peek_n(1), Some(Token::Identifier(k)) if k == "conflict")
        {
            self.advance()?; // 'on'
            self.advance()?; // 'conflict'
            let _ = self.expression()?;
            clauses.push(QueryClause::Other);
        }
        let end = self.previous_span().end;
        Ok(Expr::Query {
            clauses,
            span: start..end,
        })
    }

    /// Parses a `from <binding> in <source>` clause.
    fn query_from_clause(&mut self) -> ParseResult<QueryClause> {
        self.match_ctx_kw("from")?;
        let vars = self.query_binding_names()?;
        self.consume(Token::In, "Expected 'in' in from clause", Some("'in'"))?;
        let source = self.expression()?;
        Ok(QueryClause::From { vars, source })
    }

    /// Parses the binding of a `from`/`join` clause, returning the bound name.
    /// A leading `var` or type descriptor is accepted; destructuring patterns are
    /// consumed leniently and yield an empty name.
    fn query_binding_names(&mut self) -> ParseResult<Vec<String>> {
        self.match_token(&[Token::Var])?; // optional `var`
                                          // Destructuring binding: list `[a, b]` or mapping `{a, b}` — collect the
                                          // identifier names so they are bound in the query scope.
        if self.check(&Token::LBracket) || self.check(&Token::LBrace) {
            let close = if self.check(&Token::LBracket) {
                Token::RBracket
            } else {
                Token::RBrace
            };
            self.advance()?; // opening bracket/brace
            let mut names = Vec::new();
            while !self.check(&close) && !self.is_at_end() {
                if let Token::Identifier(n) = self.advance_owned()? {
                    names.push(n);
                }
                // A mapping field may rename (`key: binding`); the binding name is
                // what matters, so a trailing `:` binding overrides.
                if self.match_token(&[Token::Colon])? {
                    if let Some(Token::Identifier(n)) = self.peek() {
                        let n = n.clone();
                        names.pop();
                        names.push(n);
                        self.advance()?;
                    }
                }
                self.match_token(&[Token::Comma])?;
            }
            self.consume(close, "Expected closing bracket in binding pattern", None)?;
            return Ok(names);
        }
        // `<type> name` or bare `name`.
        if !(matches!(self.peek(), Some(Token::Identifier(_)))
            && matches!(self.peek_n(1), Some(Token::In)))
        {
            let _ = self.parse_type_descriptor()?;
        }
        Ok(vec![
            self.expect_ident("Expected binding name in query clause")?
        ])
    }

    /// Parses a query `let` clause (`let T x = e, ...`), without a trailing `in`.
    fn query_let_clause(&mut self) -> ParseResult<QueryClause> {
        self.consume(Token::Let, "Expected 'let'", Some("'let'"))?;
        let mut bindings = Vec::new();
        loop {
            self.match_token(&[Token::Final])?;
            // A binding is `var name` or `<type> name`.
            if !self.match_token(&[Token::Var])? {
                let _ty = self.parse_type_descriptor()?;
            }
            let name = self.expect_ident("Expected variable name in let clause")?;
            self.consume(Token::Eq, "Expected '=' in let clause", Some("'='"))?;
            let value = self.expression()?;
            bindings.push(LetBinding { name, value });
            if !self.match_token(&[Token::Comma])? {
                break;
            }
        }
        Ok(QueryClause::Let(bindings))
    }

    /// Parses a `[outer] join <binding> in <source> on <lhs> equals <rhs>` clause.
    fn query_join_clause(&mut self) -> ParseResult<QueryClause> {
        self.match_ctx_kw("outer")?;
        self.match_ctx_kw("join")?;
        let vars = self.query_binding_names()?;
        self.consume(Token::In, "Expected 'in' in join clause", Some("'in'"))?;
        let source = self.expression()?;
        self.consume(Token::On, "Expected 'on' in join clause", Some("'on'"))?;
        let on_left = self.expression()?;
        self.match_ctx_kw("equals")?;
        let on_right = self.expression()?;
        Ok(QueryClause::Join {
            vars,
            source: Box::new(source),
            on_left: Box::new(on_left),
            on_right: Box::new(on_right),
        })
    }

    /// Parses an `order by <key> [ascending|descending], ...` clause.
    fn query_order_by_clause(&mut self) -> ParseResult<QueryClause> {
        self.match_ctx_kw("order")?;
        self.match_ctx_kw("by")?;
        let mut keys = Vec::new();
        loop {
            keys.push(self.expression()?);
            // Optional direction.
            if !self.match_ctx_kw("ascending")? {
                self.match_ctx_kw("descending")?;
            }
            if !self.match_token(&[Token::Comma])? {
                break;
            }
        }
        Ok(QueryClause::OrderBy(keys))
    }

    /// Parses a table constructor: `table [key(...)] [ rows ]`.
    fn table_constructor(&mut self) -> ParseResult<Expr> {
        self.match_ctx_kw("table")?;
        let start = self.previous_span().start;
        // Optional key specifier `key(field, ...)`.
        if self.check_ctx_kw("key") && matches!(self.peek_n(1), Some(Token::LParen)) {
            self.advance()?; // 'key'
            self.advance()?; // '('
            let mut depth = 1;
            while depth > 0 {
                match self.advance_owned()? {
                    Token::LParen => depth += 1,
                    Token::RParen => depth -= 1,
                    _ => {}
                }
            }
        }
        self.consume(
            Token::LBracket,
            "Expected '[' in table constructor",
            Some("'['"),
        )?;
        let mut rows = Vec::new();
        if !self.check(&Token::RBracket) {
            loop {
                rows.push(self.expression()?);
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
        }
        self.consume(
            Token::RBracket,
            "Expected ']' after table rows",
            Some("']'"),
        )?;
        let end = self.previous_span().end;
        Ok(Expr::TableConstructor {
            rows,
            span: start..end,
        })
    }

    /// Parses an object-constructor expression, e.g. `service object { ... }`.
    /// Qualifiers and an optional `:Type` are consumed; the body is parsed into
    /// members so method bodies are reachable.
    fn object_constructor(&mut self) -> ParseResult<Expr> {
        let start = self.current_span().start;
        // Qualifiers before `object`.
        while self.match_token(&[Token::Isolated])?
            || matches!(self.peek(), Some(Token::Identifier(s)) if matches!(s.as_str(), "service" | "client"))
        {
            if matches!(self.peek(), Some(Token::Identifier(_))) {
                self.advance()?;
            }
        }
        self.consume(Token::Object, "Expected 'object'", Some("'object'"))?;
        // Optional `:TypeReference`.
        if self.match_token(&[Token::Colon])? {
            let _ = self.parse_type_descriptor()?;
        }
        let members = self.braced_block_of_members()?;
        let end = self.previous_span().end;
        Ok(Expr::ObjectConstructor {
            members,
            span: start..end,
        })
    }

    /// Parses a `start <function-call>` action expression.
    fn start_action(&mut self) -> ParseResult<Expr> {
        self.match_ctx_kw("start")?;
        let start = self.previous_span().start;
        let call = self.unary()?;
        let end = call.span().end;
        Ok(Expr::Start {
            call: Box::new(call),
            span: start..end,
        })
    }

    /// Parses a primary expression (literals, identifiers, or grouped subexpressions).
    fn primary(&mut self) -> ParseResult<Expr> {
        // Function-valued expressions must be detected before consuming a token:
        //   - `x => e`                single-parameter arrow function
        //   - `(a, b) => e`          parenthesised arrow function
        //   - `function (...) {...}` anonymous function
        if matches!(self.peek(), Some(Token::Identifier(_)))
            && matches!(self.peek_n(1), Some(Token::Arrow))
        {
            return self.arrow_function_single();
        }
        if self.check(&Token::LParen) {
            if let Some(end) = self.scan_balanced(0, &Token::LParen, &Token::RParen) {
                if matches!(self.peek_n(end), Some(Token::Arrow)) {
                    return self.arrow_function_parenthesised();
                }
            }
        }
        if (self.check(&Token::Function) && matches!(self.peek_n(1), Some(Token::LParen)))
            || (self.check(&Token::Isolated)
                && matches!(self.peek_n(1), Some(Token::Function))
                && matches!(self.peek_n(2), Some(Token::LParen)))
        {
            self.match_token(&[Token::Isolated])?; // optional `isolated` qualifier
            return self.anonymous_function();
        }
        // Typed template literal: `string \`...\``, `xml \`...\``, `re \`...\``, or
        // an identifier prefix. Treated as a string literal (interpolation
        // parsing is deferred).
        if matches!(
            self.peek(),
            Some(Token::String) | Some(Token::Identifier(_))
        ) && matches!(self.peek_n(1), Some(Token::StringTemplate(_)))
        {
            self.advance()?; // template-kind prefix
            let start = self.previous_span().start;
            if let Token::StringTemplate(s) = self.advance_owned()? {
                let end = self.previous_span().end;
                return Ok(Self::template_expr(&s, start..end));
            }
        }
        // Query expression starts with the contextual keyword `from`, optionally
        // prefixed by a collect type (`map from ...`, `table from ...`,
        // `stream from ...`).
        if self.check_ctx_kw("from") {
            return self.query_expression();
        }
        if (self.check(&Token::Map) || self.check_ctx_kw("table") || self.check_ctx_kw("stream"))
            && matches!(self.peek_n(1), Some(Token::Identifier(w)) if w == "from")
        {
            self.advance()?; // consume the collect-type keyword
            return self.query_expression();
        }
        // Table constructor: `table key(...) [...]` or `table [ { ... }, ... ]`
        // (including empty `table []`). Distinguished from indexing a variable
        // named `table` (`table[0]`) by a following `key`, `[]`, or `[{`.
        if self.check_ctx_kw("table")
            && (matches!(self.peek_n(1), Some(Token::Identifier(k)) if k == "key")
                || (matches!(self.peek_n(1), Some(Token::LBracket))
                    && matches!(self.peek_n(2), Some(Token::LBrace | Token::RBracket))))
        {
            return self.table_constructor();
        }
        // `start` action expression.
        if self.check_ctx_kw("start") {
            return self.start_action();
        }
        // Qualified reference whose module name is a keyword, e.g.
        // `transaction:onCommit(..)`, `map:keys(..)`, `object:X`. Rewritten to a
        // plain variable so the postfix `:`/call machinery handles the rest.
        if matches!(
            self.peek(),
            Some(Token::Transaction)
                | Some(Token::Map)
                | Some(Token::Object)
                | Some(Token::Function)
                | Some(Token::Type)
        ) && matches!(self.peek_n(1), Some(Token::Colon))
        {
            let name = match self.advance_owned()? {
                Token::Transaction => "transaction",
                Token::Map => "map",
                Token::Object => "object",
                Token::Function => "function",
                Token::Type => "type",
                _ => unreachable!(),
            }
            .to_string();
            let span = self.previous_span();
            return Ok(Expr::Variable { name, span });
        }
        // Object constructor expression: `[service|client|isolated] object [:T] {..}`.
        if self.check(&Token::Object)
            || (self.check(&Token::Isolated) && matches!(self.peek_n(1), Some(Token::Object)))
            || (matches!(self.peek(), Some(Token::Identifier(s)) if matches!(s.as_str(), "service" | "client"))
                && matches!(self.peek_n(1), Some(Token::Object | Token::Isolated)))
        {
            return self.object_constructor();
        }

        let token = self.advance_owned()?;
        let token_span = self.previous_span();
        match token {
            // `commit` is a transaction action expression yielding `error?`.
            Token::Commit => Ok(Expr::Variable {
                name: "commit".to_string(),
                span: token_span,
            }),
            Token::True => Ok(self.make_literal_expr(Literal::Boolean(true), token_span)),
            Token::False => Ok(self.make_literal_expr(Literal::Boolean(false), token_span)),
            Token::Number(n) => Ok(self.make_literal_expr(Literal::Number(n), token_span)),
            Token::StringLiteral(s) => Ok(self.make_literal_expr(Literal::String(s), token_span)),
            Token::StringTemplate(s) => Ok(Self::template_expr(&s, token_span)),
            Token::Identifier(name) => {
                // Check for type cast: identifier followed by backtick is `type `template``
                if matches!(self.peek(), Some(Token::StringTemplate(_))) {
                    // This is a type cast of template string
                    let template_token = self.advance_owned()?;
                    if let Token::StringTemplate(s) = template_token {
                        let end_span = self.previous_span();
                        Ok(Expr::Cast {
                            type_desc: TypeDescriptor::Basic(name),
                            expr: Box::new(
                                self.make_literal_expr(Literal::String(s), end_span.clone()),
                            ),
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
                    return Ok(
                        self.make_literal_expr(Literal::Nil, open_span.start..close_span.end)
                    );
                }
                // Inside parentheses a `:` is unambiguous (see `finish_call`).
                let was_in_branch = self.in_ternary_branch;
                self.in_ternary_branch = false;
                let expr = self.expression()?;
                self.consume(Token::RParen, "Expected ')' after expression", Some("')'"))?;
                self.in_ternary_branch = was_in_branch;
                let close_span = self.previous_span();
                Ok(self.make_grouping_expr(open_span, expr, close_span))
            }
            // Builtin type keywords used as `typedesc` values, e.g. the second
            // argument of `value:ensureType(v, string)`.
            Token::Int => Ok(self.type_value_expr("int", token_span)),
            Token::String => Ok(self.type_value_expr("string", token_span)),
            Token::Boolean => Ok(self.type_value_expr("boolean", token_span)),
            Token::Float => Ok(self.type_value_expr("float", token_span)),
            Token::Decimal => Ok(self.type_value_expr("decimal", token_span)),
            Token::Byte => Ok(self.type_value_expr("byte", token_span)),
            Token::Anydata => Ok(self.type_value_expr("anydata", token_span)),
            Token::New => {
                // Object construction: `new`, `new T(args)`, or `new (args)`.
                let start = token_span.start;
                let type_desc = if self.check(&Token::LParen) || self.check(&Token::Semicolon) {
                    None
                } else {
                    Some(self.parse_type_descriptor()?)
                };
                let mut arguments = Vec::new();
                if self.match_token(&[Token::LParen])? {
                    if !self.check(&Token::RParen) {
                        loop {
                            self.match_token(&[Token::DotDotDot])?; // spread argument
                            arguments.push(self.expression()?);
                            if !self.match_token(&[Token::Comma])? {
                                break;
                            }
                        }
                    }
                    self.consume(
                        Token::RParen,
                        "Expected ')' after constructor arguments",
                        Some("')'"),
                    )?;
                }
                let end = self.previous_span().end;
                Ok(Expr::New {
                    type_desc,
                    arguments,
                    span: start..end,
                })
            }
            Token::Lt => {
                // Type cast: <type> expression
                let type_desc = self.parse_type_descriptor()?;
                self.consume_gt("Expected '>' after cast type")?;
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

                self.consume(
                    Token::RBracket,
                    "Expected ']' after array elements",
                    Some("']'"),
                )?;
                let close_span = self.previous_span();

                Ok(Expr::ArrayLiteral {
                    elements,
                    span: open_span.start..close_span.end,
                })
            }
            Token::LBrace => {
                // Mapping constructor: `{ key: value, shorthand, ...spread, [computed]: v }`.
                let open_span = token_span;
                let mut entries = Vec::new();

                if !self.check(&Token::RBrace) {
                    loop {
                        if self.match_token(&[Token::DotDotDot])? {
                            // Spread field `...expr`.
                            let value = self.expression()?;
                            entries.push(("...".to_string(), value));
                        } else if self.match_token(&[Token::LBracket])? {
                            // Computed key `[expr]: value` — key expression accepted
                            // but not retained.
                            let _key = self.expression()?;
                            self.consume(
                                Token::RBracket,
                                "Expected ']' after computed key",
                                Some("']'"),
                            )?;
                            self.consume(
                                Token::Colon,
                                "Expected ':' after computed key",
                                Some("':'"),
                            )?;
                            let value = self.expression()?;
                            entries.push((String::new(), value));
                        } else {
                            let key =
                                self.expect_ident_or_string("Expected key in mapping constructor")?;
                            let key_span = self.previous_span();
                            if self.match_token(&[Token::Colon])? {
                                let value = self.expression()?;
                                entries.push((key, value));
                            } else {
                                // Field shorthand `{ name }` == `{ name: name }`.
                                let value = Expr::Variable {
                                    name: key.clone(),
                                    span: key_span,
                                };
                                entries.push((key, value));
                            }
                        }

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

    /// Parses a type descriptor.
    ///
    /// Structured as a precedence climb so binding strength matches the spec:
    /// union `|` (loosest) → intersection `&` → postfix (`[]`, `?`) → primary.
    fn parse_type_descriptor(&mut self) -> ParseResult<TypeDescriptor> {
        self.parse_type_union()
    }

    /// Parses union types (`T1 | T2 | ...`), the loosest-binding type operator.
    fn parse_type_union(&mut self) -> ParseResult<TypeDescriptor> {
        let mut left = self.parse_type_intersection()?;
        while self.match_token(&[Token::Pipe])? {
            let mut members = match left {
                TypeDescriptor::Union(members) => members,
                other => vec![other],
            };
            members.push(self.parse_type_intersection()?);
            left = TypeDescriptor::Union(members);
        }
        Ok(left)
    }

    /// Parses intersection types (`T1 & T2 & ...`).
    fn parse_type_intersection(&mut self) -> ParseResult<TypeDescriptor> {
        let mut left = self.parse_type_postfix()?;
        while self.match_token(&[Token::Amp])? {
            let mut members = match left {
                TypeDescriptor::Intersection(members) => members,
                other => vec![other],
            };
            members.push(self.parse_type_postfix()?);
            left = TypeDescriptor::Intersection(members);
        }
        Ok(left)
    }

    /// Parses array (`[]`, `[n]`, `[*]`) and optional (`?`) type suffixes.
    fn parse_type_postfix(&mut self) -> ParseResult<TypeDescriptor> {
        let mut type_desc = self.parse_type_primary()?;
        loop {
            // A following `[` is an array suffix only if it holds an array
            // dimension; `[a, b]` after a type is a binding pattern, not a suffix.
            if self.check(&Token::LBracket) && self.bracket_is_binding_pattern() {
                break;
            }
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
                self.consume(
                    Token::RBracket,
                    "Expected ']' after array dimension",
                    Some("']'"),
                )?;
                type_desc = TypeDescriptor::Array {
                    element_type: Box::new(type_desc),
                    dimension,
                };
            } else if self.check(&Token::Question)
                && !(self.in_is_type && self.question_starts_ternary())
            {
                self.advance()?; // '?'
                type_desc = TypeDescriptor::Optional(Box::new(type_desc));
            } else {
                break;
            }
        }
        Ok(type_desc)
    }

    /// Returns true when the `[` at the cursor opens a binding pattern rather
    /// than an array-dimension suffix. An array suffix is empty (`[]`), or holds
    /// a single dimension (`[3]`, `[*]`, `[CONST]`); anything else — notably a
    /// comma-separated list like `[name, grade]` — is a binding pattern.
    fn bracket_is_binding_pattern(&self) -> bool {
        match self.peek_n(1) {
            Some(Token::RBracket) | Some(Token::Star) | Some(Token::Number(_)) => false,
            Some(Token::Identifier(_)) => {
                // `[CONST]` is a dimension; `[a, b]` / `[a]` followed by `in` is a
                // binding pattern.
                !matches!(self.peek_n(2), Some(Token::RBracket))
                    || matches!(self.peek_n(3), Some(Token::In))
            }
            _ => true,
        }
    }

    /// Heuristic used inside type parsing to tell an optional-type suffix `T?`
    /// from a conditional expression `... is T ? a : b`: if the token after `?`
    /// can begin an expression, the `?` belongs to a ternary and must not be
    /// consumed as a type suffix.
    fn question_starts_ternary(&self) -> bool {
        // `{` and `[` are deliberately excluded: after `?` they are ambiguous with
        // a block/body (`returns T? {`) or an array suffix (`int?[]`), which must
        // win over a rare ternary whose true-branch is a constructor.
        matches!(
            self.peek_n(1),
            Some(Token::Identifier(_))
                | Some(Token::Number(_))
                | Some(Token::StringLiteral(_))
                | Some(Token::StringTemplate(_))
                | Some(Token::True)
                | Some(Token::False)
                | Some(Token::LParen)
                | Some(Token::Minus)
                | Some(Token::Bang)
                | Some(Token::Tilde)
                | Some(Token::Lt)
                | Some(Token::Check)
                | Some(Token::Checkpanic)
                | Some(Token::Trap)
                | Some(Token::New)
                | Some(Token::Typeof)
                | Some(Token::Let)
        )
    }

    /// Parses a primary (atomic) type descriptor: builtins, `map<T>`, generics,
    /// records, objects, tuples, function types, singletons, `distinct`, and
    /// named/qualified type references.
    fn parse_type_primary(&mut self) -> ParseResult<TypeDescriptor> {
        // Annotations may prefix a type in return/parameter/field position, e.g.
        // `returns @http:Cache Payload` — skip them.
        self.skip_annotations()?;
        // Type-level qualifiers that precede object/function types
        // (`isolated function`, `service object`, `client object`, `readonly`).
        while self.match_token(&[Token::Isolated])?
            || matches!(self.peek(), Some(Token::Identifier(s)) if matches!(s.as_str(), "service" | "client" | "transactional"))
        {
            if matches!(self.peek(), Some(Token::Identifier(_))) {
                self.advance()?;
            }
        }
        // map<T>
        if self.match_token(&[Token::Map])? {
            self.consume(Token::Lt, "Expected '<' after 'map'", Some("'<'"))?;
            let value_type = Box::new(self.parse_type_descriptor()?);
            self.consume_gt("Expected '>' after map value type")?;
            return Ok(TypeDescriptor::Map { value_type });
        }
        if self.check(&Token::Function) {
            return self.parse_function_type();
        }
        if self.check(&Token::Record) {
            return self.parse_record_type();
        }
        // `object { .. }` inline type, but `object:Name` is a qualified reference
        // to a type in the `object` lang library.
        if self.check(&Token::Object) && !matches!(self.peek_n(1), Some(Token::Colon)) {
            return self.parse_object_type();
        }
        // Lang-library qualified references where the module name is a keyword,
        // e.g. `object:RawTemplate`, `map:Entry`, `function:Type`.
        if matches!(
            self.peek(),
            Some(Token::Object)
                | Some(Token::Map)
                | Some(Token::Function)
                | Some(Token::Type)
                | Some(Token::Transaction)
        ) && matches!(self.peek_n(1), Some(Token::Colon))
        {
            let module = match self.advance_owned()? {
                Token::Object => "object",
                Token::Map => "map",
                Token::Function => "function",
                Token::Type => "type",
                Token::Transaction => "transaction",
                _ => unreachable!(),
            }
            .to_string();
            self.advance()?; // ':'
            let name = self.expect_ident("Expected type name after ':'")?;
            return Ok(TypeDescriptor::Qualified { module, name });
        }
        if self.check(&Token::LBracket) {
            return self.parse_tuple_type();
        }
        // Nil type `()`.
        if self.check(&Token::LParen) && matches!(self.peek_n(1), Some(Token::RParen)) {
            self.advance()?;
            self.advance()?;
            return Ok(TypeDescriptor::Basic("()".to_string()));
        }
        // Singleton literal types: 1, -1, "OPEN", true, false
        if matches!(
            self.peek(),
            Some(Token::Number(_))
                | Some(Token::StringLiteral(_))
                | Some(Token::True)
                | Some(Token::False)
                | Some(Token::Minus)
        ) {
            return self.parse_singleton_type();
        }

        let token = self.advance_owned()?;
        let base = match token {
            Token::Int => TypeDescriptor::Basic("int".to_string()),
            Token::String => TypeDescriptor::Basic("string".to_string()),
            Token::Boolean => TypeDescriptor::Basic("boolean".to_string()),
            Token::Float => TypeDescriptor::Basic("float".to_string()),
            Token::Decimal => TypeDescriptor::Basic("decimal".to_string()),
            Token::Byte => TypeDescriptor::Basic("byte".to_string()),
            Token::Anydata => TypeDescriptor::Basic("anydata".to_string()),
            Token::Identifier(name) => {
                // `distinct T` prefix (contextual keyword).
                if name == "distinct" {
                    let inner = self.parse_type_primary()?;
                    return Ok(TypeDescriptor::Distinct(Box::new(inner)));
                }
                // Module-qualified reference `mod:Type`.
                if self.check(&Token::Colon) && matches!(self.peek_n(1), Some(Token::Identifier(_)))
                {
                    self.advance()?; // ':'
                    let member = self.expect_ident("Expected type name after ':'")?;
                    TypeDescriptor::Qualified {
                        module: name,
                        name: member,
                    }
                } else {
                    TypeDescriptor::Basic(name)
                }
            }
            t => {
                return Err(
                    self.error_previous(&format!("Expected type, found {:?}", t), Some("type"))
                )
            }
        };

        // A builtin type may be qualified into a subtype: `int:Unsigned32`,
        // `string:Char`, `xml:Element`, etc.
        let base = if let TypeDescriptor::Basic(n) = &base {
            if self.check(&Token::Colon) && matches!(self.peek_n(1), Some(Token::Identifier(_))) {
                let module = n.clone();
                self.advance()?; // ':'
                let member = self.expect_ident("Expected type name after ':'")?;
                TypeDescriptor::Qualified {
                    module,
                    name: member,
                }
            } else {
                base
            }
        } else {
            base
        };

        // Optional generic type arguments on a named/qualified type.
        if self.check(&Token::Lt) {
            let name = match &base {
                TypeDescriptor::Basic(n) => n.clone(),
                TypeDescriptor::Qualified { module, name } => format!("{module}:{name}"),
                _ => unreachable!("generic base is always named"),
            };
            self.advance()?; // '<'
            let mut args = Vec::new();
            loop {
                args.push(self.parse_type_descriptor()?);
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
            self.consume_gt("Expected '>' after type arguments")?;
            // `table<R> key(field, ...)` or `table<R> key<[K1, K2]>` carries a
            // trailing key specifier; accept and discard it (parse-tolerant).
            if matches!(self.peek(), Some(Token::Identifier(s)) if s == "key") {
                if matches!(self.peek_n(1), Some(Token::LParen)) {
                    self.advance()?; // 'key'
                    self.advance()?; // '('
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance_owned()? {
                            Token::LParen => depth += 1,
                            Token::RParen => depth -= 1,
                            _ => {}
                        }
                    }
                } else if matches!(self.peek_n(1), Some(Token::Lt)) {
                    self.advance()?; // 'key'
                    self.advance()?; // '<'
                    let _ = self.parse_type_descriptor()?;
                    self.consume_gt("Expected '>' after key type")?;
                }
            }
            return Ok(TypeDescriptor::Generic { name, args });
        }

        Ok(base)
    }

    /// Parses a function type descriptor `function (params) returns T`.
    fn parse_function_type(&mut self) -> ParseResult<TypeDescriptor> {
        self.consume(Token::Function, "Expected 'function'", Some("'function'"))?;
        let mut params = Vec::new();
        if self.match_token(&[Token::LParen])? {
            while !self.check(&Token::RParen) {
                self.skip_annotations()?;
                self.match_token(&[Token::Public])?; // rare visibility qualifier
                let param_type = self.parse_type_descriptor()?;
                self.match_token(&[Token::DotDotDot])?; // rest param marker
                                                        // Optional parameter name.
                if matches!(self.peek(), Some(Token::Identifier(_))) {
                    self.advance()?;
                }
                // Optional default value `= expr`.
                if self.match_token(&[Token::Eq])? {
                    let _ = self.expression()?;
                }
                params.push(param_type);
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
            self.consume(
                Token::RParen,
                "Expected ')' after function type parameters",
                Some("')'"),
            )?;
        }
        let return_type = if self.match_token(&[Token::Returns])? {
            Some(Box::new(self.parse_type_descriptor()?))
        } else {
            None
        };
        Ok(TypeDescriptor::Function {
            params,
            return_type,
        })
    }

    /// Parses a tuple type descriptor `[T1, T2, ...R]`.
    fn parse_tuple_type(&mut self) -> ParseResult<TypeDescriptor> {
        self.consume(Token::LBracket, "Expected '['", Some("'['"))?;
        let mut members = Vec::new();
        let mut rest = None;
        if !self.check(&Token::RBracket) {
            loop {
                let member = self.parse_type_descriptor()?;
                if self.match_token(&[Token::DotDotDot])? {
                    // Rest member `T...` must be last.
                    rest = Some(Box::new(member));
                    break;
                }
                members.push(member);
                if !self.match_token(&[Token::Comma])? {
                    break;
                }
            }
        }
        self.consume(
            Token::RBracket,
            "Expected ']' after tuple members",
            Some("']'"),
        )?;
        Ok(TypeDescriptor::Tuple { members, rest })
    }

    /// Parses an inline record type `record { ... }` / `record {| ... |}`.
    fn parse_record_type(&mut self) -> ParseResult<TypeDescriptor> {
        self.consume(Token::Record, "Expected 'record'", Some("'record'"))?;
        self.consume(Token::LBrace, "Expected '{' after 'record'", Some("'{'"))?;
        let closed = self.match_token(&[Token::Pipe])?; // `{|`
        let mut fields = Vec::new();
        let mut rest = None;
        loop {
            // Closing delimiter.
            if closed {
                if self.check(&Token::Pipe) && matches!(self.peek_n(1), Some(Token::RBrace)) {
                    self.advance()?; // '|'
                    self.advance()?; // '}'
                    break;
                }
            } else if self.match_token(&[Token::RBrace])? {
                break;
            }
            if self.is_at_end() {
                return Err(self.unexpected_eof(Some("'}'")));
            }

            // Type inclusion `*T;`.
            if self.match_token(&[Token::Star])? {
                let _included = self.parse_type_descriptor()?;
                self.consume(
                    Token::Semicolon,
                    "Expected ';' after record type inclusion",
                    Some("';'"),
                )?;
                continue;
            }

            // Optional `readonly` field qualifier (but not the `readonly & T`
            // intersection type, which parse_type_descriptor handles).
            if self.check_ctx_kw("readonly") && !matches!(self.peek_n(1), Some(Token::Amp)) {
                self.advance()?;
            }
            let field_type = self.parse_type_descriptor()?;
            // Rest field `T...;`.
            if self.match_token(&[Token::DotDotDot])? {
                rest = Some(Box::new(field_type));
                self.consume(
                    Token::Semicolon,
                    "Expected ';' after record rest field",
                    Some("';'"),
                )?;
                continue;
            }
            let name = self.expect_ident("Expected field name in record type")?;
            let optional = self.match_token(&[Token::Question])?;
            if self.match_token(&[Token::Eq])? {
                let _default = self.expression()?;
            }
            self.consume(
                Token::Semicolon,
                "Expected ';' after record field",
                Some("';'"),
            )?;
            fields.push(RecordField {
                name,
                type_desc: field_type,
                optional,
            });
        }
        Ok(TypeDescriptor::Record {
            fields,
            rest,
            closed,
        })
    }

    /// Parses an object type `object { ... }`. The body is consumed leniently
    /// (balanced braces) and not retained; this keeps type positions tolerant of
    /// objects without a full object-member grammar.
    fn parse_object_type(&mut self) -> ParseResult<TypeDescriptor> {
        self.consume(Token::Object, "Expected 'object'", Some("'object'"))?;
        self.skip_braced_block()?;
        Ok(TypeDescriptor::Object)
    }

    /// Parses a singleton (literal) type such as `1`, `-2`, `"OPEN"`, `true`.
    fn parse_singleton_type(&mut self) -> ParseResult<TypeDescriptor> {
        let mut text = String::new();
        if self.match_token(&[Token::Minus])? {
            text.push('-');
        }
        let token = self.advance_owned()?;
        let literal = match token {
            Token::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", n as i64)
                } else {
                    n.to_string()
                }
            }
            Token::StringLiteral(s) => format!("\"{s}\""),
            Token::True => "true".to_string(),
            Token::False => "false".to_string(),
            t => {
                return Err(self.error_previous(
                    &format!("Expected literal in singleton type, found {:?}", t),
                    Some("literal"),
                ))
            }
        };
        text.push_str(&literal);
        Ok(TypeDescriptor::Singleton(text))
    }

    /// Consumes a balanced `{ ... }` block, discarding its contents. Used for
    /// lenient parse-tolerant handling of object type bodies.
    fn skip_braced_block(&mut self) -> ParseResult<()> {
        self.consume(Token::LBrace, "Expected '{'", Some("'{'"))?;
        let mut depth = 1;
        while depth > 0 {
            match self.advance_owned()? {
                Token::LBrace => depth += 1,
                Token::RBrace => depth -= 1,
                _ => {}
            }
        }
        Ok(())
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

    /// Builds a reference to a builtin type used as a `typedesc` value in
    /// expression position.
    fn type_value_expr(&self, name: &str, span: Span) -> Expr {
        Expr::Variable {
            name: name.to_string(),
            span,
        }
    }

    /// Builds a `StringTemplate` expression, parsing each `${...}` interpolation
    /// into a sub-expression so variable references inside templates are tracked.
    /// Interpolation sub-expressions carry template-relative spans and are used
    /// only for reference tracking, not for span-accurate diagnostics.
    fn template_expr(content: &str, span: Span) -> Expr {
        let mut interpolations = Vec::new();
        let bytes = content.as_bytes();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] == b'$' && bytes[i + 1] == b'{' {
                let start = i + 2;
                let mut depth = 1;
                let mut j = start;
                while j < bytes.len() {
                    match bytes[j] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                let inner = &content[start..j.min(content.len())];
                if let Ok(tokens) = crate::lexer::Lexer::new(inner).collect::<Result<Vec<_>, _>>() {
                    if !tokens.is_empty() {
                        let mut sub = Parser::new(tokens);
                        if let Ok(expr) = sub.expression() {
                            interpolations.push(expr);
                        }
                    }
                }
                i = j + 1;
            } else {
                i += 1;
            }
        }
        Expr::StringTemplate {
            interpolations,
            span,
        }
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

    /// Returns true when the current token is the given contextual keyword
    /// (an identifier with matching text). Query/table keywords are contextual so
    /// they remain usable as ordinary identifiers elsewhere.
    fn check_ctx_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Identifier(s)) if s == kw)
    }

    /// Consumes the current token if it is the given contextual keyword.
    fn match_ctx_kw(&mut self, kw: &str) -> ParseResult<bool> {
        if self.check_ctx_kw(kw) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
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

    /// Consumes a single closing `>` in type-argument position.
    ///
    /// The lexer greedily merges consecutive `>` characters into `GtGt`/`GtGtGt`
    /// (and `>=` into `Ge`), which breaks naive `consume(Token::Gt, ..)` calls
    /// inside nested generics/casts such as `map<int[]>` or `stream<int,error>`.
    /// This splits the compound token in place: it logically consumes one `>` and
    /// rewrites the pending token to the smaller remainder (with an adjusted start
    /// offset) instead of advancing past all of it.
    fn consume_gt(&mut self, msg: &str) -> ParseResult<()> {
        match self.peek() {
            Some(Token::Gt) => {
                self.advance()?;
                Ok(())
            }
            Some(Token::GtGt) => {
                let (start, _, end) = self.tokens[self.current];
                self.tokens[self.current] = (start + 1, Token::Gt, end);
                Ok(())
            }
            Some(Token::GtGtGt) => {
                let (start, _, end) = self.tokens[self.current];
                self.tokens[self.current] = (start + 1, Token::GtGt, end);
                Ok(())
            }
            Some(Token::Ge) => {
                let (start, _, end) = self.tokens[self.current];
                self.tokens[self.current] = (start + 1, Token::Eq, end);
                Ok(())
            }
            _ => Err(self.error_here(msg, Some("'>'"))),
        }
    }

    /// Scans forward from `start_offset` (which must point at an `open` token) to
    /// the matching `close`, accounting for nesting, and returns the offset just
    /// past the close. Returns `None` if the delimiters are unbalanced or the
    /// stream ends first. Purely a lookahead helper — it never consumes tokens.
    fn scan_balanced(&self, start_offset: usize, open: &Token, close: &Token) -> Option<usize> {
        let mut depth = 0usize;
        let mut offset = start_offset;
        loop {
            match self.peek_n(offset) {
                Some(token) if token == open => {
                    depth += 1;
                    offset += 1;
                }
                Some(token) if token == close => {
                    offset += 1;
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(offset);
                    }
                }
                Some(_) => offset += 1,
                None => return None,
            }
        }
    }

    /// Consumes the current token, requiring it to be an identifier, and returns
    /// the identifier's text. Collapses the repeated `advance_owned()` + match
    /// pattern used throughout the parser.
    fn expect_ident(&mut self, msg: &str) -> ParseResult<String> {
        let token = self.advance_owned()?;
        match token {
            Token::Identifier(name) => Ok(name),
            _ => Err(self.error_previous(msg, Some("identifier"))),
        }
    }

    /// Consumes the current token, requiring it to be an identifier or a string
    /// literal, and returns its text. Used for keys that may be quoted.
    fn expect_ident_or_string(&mut self, msg: &str) -> ParseResult<String> {
        let token = self.advance_owned()?;
        match token {
            Token::Identifier(name) => Ok(name),
            Token::StringLiteral(s) => Ok(s),
            _ => Err(self.error_previous(msg, Some("identifier or string"))),
        }
    }

    /// Determines whether the upcoming tokens form the start of a variable
    /// declaration (`<type> <ident>` or `var`/`final`/`const`).
    ///
    /// Uses bounded lookahead (`skip_type`) to walk past an arbitrary type
    /// descriptor and check whether an identifier (the binding name) follows.
    /// This is how the recursive-descent parser resolves the ambiguity that the
    /// reference LALR grammar handles with a table (e.g. `a < b;` comparison vs
    /// `Stream<int> s;` typed binding).
    fn starts_var_decl(&self) -> bool {
        // A leading `from` opens a query expression, not a `<type> <name>` binding.
        if self.check_ctx_kw("from") {
            return false;
        }
        match self.peek() {
            Some(Token::Var) | Some(Token::Final) | Some(Token::Const) => true,
            Some(Token::LBracket) => {
                // Tuple-typed binding `[T1, T2][] name` vs an array-literal
                // expression statement `[1, 2]...`: only the former is a type
                // (possibly with array/optional suffixes) followed by an identifier.
                self.skip_type(0)
                    .is_some_and(|end| matches!(self.peek_n(end), Some(Token::Identifier(_))))
            }
            Some(token)
                if Self::is_type_start(token)
                    || matches!(
                        token,
                        Token::Identifier(_)
                            | Token::Record
                            | Token::Object
                            | Token::StringLiteral(_)
                    ) =>
            {
                self.skip_type(0)
                    .is_some_and(|end| matches!(self.peek_n(end), Some(Token::Identifier(_))))
            }
            _ => false,
        }
    }

    /// Determines whether a `const` declaration carries a type annotation, i.e.
    /// `const <type> <name>` rather than `const <name>`.
    fn const_has_type_annotation(&self) -> bool {
        match self.peek() {
            Some(token) if Self::is_type_start(token) => true,
            Some(Token::Identifier(_)) => self
                .skip_type(0)
                .is_some_and(|end| matches!(self.peek_n(end), Some(Token::Identifier(_)))),
            _ => false,
        }
    }

    /// Lookahead helper: assuming `peek_n(start)` begins a type descriptor,
    /// returns the offset just past that descriptor, or `None` if the tokens do
    /// not form a plausible type. Purely non-consuming; used to disambiguate
    /// declarations from expressions.
    fn skip_type(&self, start: usize) -> Option<usize> {
        let mut offset = start;
        // Primary type.
        match self.peek_n(offset)? {
            Token::Int
            | Token::String
            | Token::Boolean
            | Token::Float
            | Token::Decimal
            | Token::Byte
            | Token::Anydata
            | Token::Map
            | Token::Identifier(_) => offset += 1,
            Token::Record | Token::Object => {
                offset += 1;
                if matches!(self.peek_n(offset), Some(Token::LBrace)) {
                    offset = self.scan_balanced(offset, &Token::LBrace, &Token::RBrace)?;
                } else {
                    return None;
                }
            }
            // Tuple type `[T1, T2, ...]`.
            Token::LBracket => {
                offset = self.scan_balanced(offset, &Token::LBracket, &Token::RBracket)?;
            }
            // Singleton literal type, e.g. `"off" arg = "off";` or `1|2 x = 1;`.
            Token::StringLiteral(_) | Token::Number(_) | Token::True | Token::False => offset += 1,
            _ => return None,
        }
        // Module qualification `mod:Type`.
        if matches!(self.peek_n(offset), Some(Token::Colon))
            && matches!(self.peek_n(offset + 1), Some(Token::Identifier(_)))
        {
            offset += 2;
        }
        // Generic arguments `<...>`.
        if matches!(self.peek_n(offset), Some(Token::Lt)) {
            offset = self.scan_angle(offset)?;
        }
        // Suffixes: array `[...]`, optional `?`, union `|T`, intersection `&T`.
        loop {
            match self.peek_n(offset) {
                Some(Token::LBracket) => {
                    offset = self.scan_balanced(offset, &Token::LBracket, &Token::RBracket)?;
                }
                Some(Token::Question) => offset += 1,
                Some(Token::Pipe) | Some(Token::Amp) => {
                    offset = self.skip_type(offset + 1)?;
                }
                _ => break,
            }
        }
        Some(offset)
    }

    /// Lookahead helper: assuming `peek_n(start)` is `<`, returns the offset just
    /// past the matching `>`, accounting for nesting and for the lexer merging
    /// consecutive `>` into `GtGt`/`GtGtGt`. Non-consuming.
    fn scan_angle(&self, start: usize) -> Option<usize> {
        let mut depth: i32 = 0;
        let mut offset = start;
        loop {
            match self.peek_n(offset)? {
                Token::Lt => depth += 1,
                Token::LtLt => depth += 2,
                Token::Gt | Token::Ge => depth -= 1,
                Token::GtGt => depth -= 2,
                Token::GtGtGt => depth -= 3,
                _ => {}
            }
            offset += 1;
            if depth <= 0 {
                return Some(offset);
            }
        }
    }

    /// Returns true when the token can begin a simple type descriptor in our subset.
    fn is_type_start(token: &Token) -> bool {
        matches!(
            token,
            Token::Int
                | Token::String
                | Token::Boolean
                | Token::Float
                | Token::Decimal
                | Token::Byte
                | Token::Anydata
                | Token::Map
        )
    }
}

#[cfg(test)]
mod tests {
    //! Structural (span-insensitive) parser tests.
    //!
    //! These assert on AST *shape* via `matches!` and counts rather than on
    //! `{:#?}` output or byte offsets, so they remain stable as spans shift while
    //! the grammar is extended. They characterise current behaviour and act as the
    //! regression net for the grammar-coverage expansion (SRS risk R-01).
    use super::*;
    use crate::lexer::Lexer;

    /// Lexes and parses a source string, returning the AST and any diagnostics.
    fn parse(src: &str) -> (Vec<Stmt>, Vec<Diagnostic>) {
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<_>, _>>()
            .expect("source should lex without errors");
        Parser::new(tokens).parse()
    }

    /// Parses a source string, asserting there are no parse diagnostics, and
    /// returns the statements.
    fn parse_ok(src: &str) -> Vec<Stmt> {
        let (stmts, diags) = parse(src);
        assert!(
            diags.is_empty(),
            "expected no parse errors, got: {:?}",
            diags
        );
        stmts
    }

    #[test]
    fn parses_import() {
        let stmts = parse_ok("import ballerina/io;");
        assert_eq!(stmts.len(), 1);
        assert!(matches!(&stmts[0], Stmt::Import { package_path, .. } if package_path.len() == 2));
    }

    #[test]
    fn parses_typed_var_decl() {
        let stmts = parse_ok("int x = 5;");
        assert!(matches!(
            &stmts[0],
            Stmt::VarDecl {
                type_annotation: Some(_),
                initializer: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_final_and_var_keyword_decls() {
        assert!(matches!(
            &parse_ok("final int x = 1;")[0],
            Stmt::VarDecl { is_final: true, .. }
        ));
        assert!(matches!(
            &parse_ok("var y = 2;")[0],
            Stmt::VarDecl {
                type_annotation: None,
                ..
            }
        ));
    }

    #[test]
    fn parses_const_decl() {
        assert!(matches!(
            &parse_ok("const MAX = 100;")[0],
            Stmt::ConstDecl { .. }
        ));
    }

    #[test]
    fn parses_function_with_params_and_return() {
        let stmts = parse_ok("function add(int a, int b) returns int { return a + b; }");
        match &stmts[0] {
            Stmt::Function {
                params,
                return_type: Some(_),
                body,
                is_public: false,
                ..
            } => {
                assert_eq!(params.len(), 2);
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected function, got {:?}", other),
        }
    }

    #[test]
    fn parses_public_function() {
        assert!(matches!(
            &parse_ok("public function main() { }")[0],
            Stmt::Function {
                is_public: true,
                ..
            }
        ));
    }

    #[test]
    fn parses_if_else() {
        let stmts = parse_ok("function f() { if (true) { } else { } }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::If {
                else_branch: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_while_and_foreach() {
        let stmts = parse_ok("function f() { while (true) { } foreach int i in x { } }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(&body[0], Stmt::While { .. }));
        assert!(matches!(&body[1], Stmt::Foreach { .. }));
    }

    #[test]
    fn arithmetic_precedence_is_multiplicative_over_additive() {
        // `1 + 2 * 3` should parse as `1 + (2 * 3)`.
        let stmts = parse_ok("int x = 1 + 2 * 3;");
        let Stmt::VarDecl {
            initializer: Some(Expr::Binary { op, right, .. }),
            ..
        } = &stmts[0]
        else {
            panic!("expected binary initializer")
        };
        assert!(matches!(op, BinaryOp::Plus));
        assert!(matches!(
            right.as_ref(),
            Expr::Binary {
                op: BinaryOp::Star,
                ..
            }
        ));
    }

    #[test]
    fn parses_array_and_map_literals() {
        assert!(matches!(
            &parse_ok("int[] a = [1, 2, 3];")[0],
            Stmt::VarDecl {
                initializer: Some(Expr::ArrayLiteral { .. }),
                ..
            }
        ));
        assert!(matches!(
            &parse_ok("map<string> m = {name: \"x\"};")[0],
            Stmt::VarDecl {
                initializer: Some(Expr::MapLiteral { .. }),
                ..
            }
        ));
    }

    #[test]
    fn parses_ternary_and_elvis() {
        let ternary = parse_ok("int x = c ? 1 : 0;");
        assert!(matches!(
            &ternary[0],
            Stmt::VarDecl {
                initializer: Some(Expr::Ternary { .. }),
                ..
            }
        ));
        let elvis = parse_ok("int y = a ?: 0;");
        assert!(matches!(
            &elvis[0],
            Stmt::VarDecl {
                initializer: Some(Expr::Elvis { .. }),
                ..
            }
        ));
    }

    #[test]
    fn parses_cast_expression() {
        assert!(matches!(
            &parse_ok("int x = <int>y;")[0],
            Stmt::VarDecl {
                initializer: Some(Expr::Cast { .. }),
                ..
            }
        ));
    }

    #[test]
    fn parses_qualified_call() {
        let stmts = parse_ok("function f() { io:println(\"hi\"); }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::Expression {
                expression: Expr::Call { .. },
                ..
            }
        ));
    }

    #[test]
    fn parses_method_call_and_member_access() {
        let stmts = parse_ok("function f() { x.push(1); int a = arr[0]; }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::Expression {
                expression: Expr::MethodCall { .. },
                ..
            }
        ));
        assert!(matches!(
            &body[1],
            Stmt::VarDecl {
                initializer: Some(Expr::MemberAccess { .. }),
                ..
            }
        ));
    }

    #[test]
    fn reports_error_on_missing_semicolon() {
        let (_stmts, diags) = parse("int x = 5");
        assert!(!diags.is_empty(), "missing ';' should produce a diagnostic");
    }

    #[test]
    fn recovers_and_reports_multiple_errors() {
        // Two bad statements; recovery should let the parser find both.
        let (_stmts, diags) = parse("function f() { int x = ; int y = ; }");
        assert!(!diags.is_empty());
    }

    /// Builds a parser over the tokens of `src` for direct helper testing.
    fn parser_for(src: &str) -> Parser {
        let tokens = Lexer::new(src)
            .collect::<Result<Vec<_>, _>>()
            .expect("source should lex without errors");
        Parser::new(tokens)
    }

    #[test]
    fn consume_gt_splits_compound_close_tokens() {
        // `>>` lexes as a single GtGt; consume_gt must split it into two `>`.
        let mut p = parser_for(">>");
        assert!(p.consume_gt("first").is_ok());
        assert!(p.consume_gt("second").is_ok());
        assert!(p.is_at_end());

        // `>>>` (GtGtGt) splits into three `>`.
        let mut p = parser_for(">>>");
        assert!(p.consume_gt("1").is_ok());
        assert!(p.consume_gt("2").is_ok());
        assert!(p.consume_gt("3").is_ok());
        assert!(p.is_at_end());

        // `>=` (Ge) yields one `>` and leaves an `=`.
        let mut p = parser_for(">=");
        assert!(p.consume_gt("gt").is_ok());
        assert!(matches!(p.peek(), Some(Token::Eq)));

        // A non-`>` token is an error.
        let mut p = parser_for("x");
        assert!(p.consume_gt("expected gt").is_err());
    }

    #[test]
    fn scan_balanced_finds_matching_close() {
        // Tokens: [ ( ) ( ) ] x  — from offset 0, the matching ] is at index 5,
        // so scan_balanced returns 6 (offset just past it).
        let p = parser_for("[()()] x");
        assert_eq!(
            p.scan_balanced(0, &Token::LBracket, &Token::RBracket),
            Some(6)
        );
        // Nested brackets.
        let p = parser_for("[[1], [2]]");
        assert_eq!(
            p.scan_balanced(0, &Token::LBracket, &Token::RBracket),
            Some(9)
        );
        // Unbalanced returns None.
        let p = parser_for("[1, 2");
        assert_eq!(p.scan_balanced(0, &Token::LBracket, &Token::RBracket), None);
    }

    #[test]
    fn expect_ident_returns_name_or_errors() {
        let mut p = parser_for("hello");
        assert_eq!(p.expect_ident("want ident").unwrap(), "hello");

        let mut p = parser_for("123");
        assert!(p.expect_ident("want ident").is_err());
    }

    // ---- Phase 1: type-descriptor coverage ------------------------------------

    /// Extracts the type annotation from a single top-level `Type name = init;`.
    fn var_type(src: &str) -> TypeDescriptor {
        match &parse_ok(src)[0] {
            Stmt::VarDecl {
                type_annotation: Some(ty),
                ..
            } => ty.clone(),
            other => panic!("expected typed var decl, got {:?}", other),
        }
    }

    #[test]
    fn parses_tuple_type() {
        assert!(matches!(
            var_type("[int, string] pair = [1, \"a\"];"),
            TypeDescriptor::Tuple { members, rest: None } if members.len() == 2
        ));
    }

    #[test]
    fn parses_tuple_type_with_rest() {
        assert!(matches!(
            var_type("[int, string...] t = x;"),
            TypeDescriptor::Tuple { rest: Some(_), .. }
        ));
    }

    #[test]
    fn parses_open_record_type() {
        let ty = var_type("record { int id; string name; } r = x;");
        assert!(matches!(
            ty,
            TypeDescriptor::Record { fields, closed: false, .. } if fields.len() == 2
        ));
    }

    #[test]
    fn parses_closed_record_with_optional_and_default() {
        let ty = var_type("record {| int id; string name?; int count = 0; |} r = x;");
        match ty {
            TypeDescriptor::Record {
                fields,
                closed: true,
                ..
            } => {
                assert_eq!(fields.len(), 3);
                assert!(fields.iter().any(|f| f.name == "name" && f.optional));
            }
            other => panic!("expected closed record, got {:?}", other),
        }
    }

    #[test]
    fn parses_generic_types() {
        assert!(matches!(
            var_type("stream<int> s = x;"),
            TypeDescriptor::Generic { name, args } if name == "stream" && args.len() == 1
        ));
        assert!(matches!(
            var_type("map<int> m = x;"),
            TypeDescriptor::Map { .. }
        ));
        // Nested generics that close with `>>` exercise consume_gt.
        assert!(matches!(
            var_type("stream<map<int>> s = x;"),
            TypeDescriptor::Generic { .. }
        ));
    }

    #[test]
    fn parses_intersection_type() {
        assert!(matches!(
            var_type("readonly & Config c = x;"),
            TypeDescriptor::Intersection(members) if members.len() == 2
        ));
    }

    #[test]
    fn parses_singleton_and_union_of_singletons() {
        // Singleton types realistically appear in return/param positions (parsed
        // directly via parse_type_descriptor), e.g. an enum-like union return.
        let stmts = parse_ok("function f() returns 1|2|3 { return 1; }");
        assert!(matches!(
            &stmts[0],
            Stmt::Function {
                return_type: Some(TypeDescriptor::Union(members)),
                ..
            } if members.len() == 3
        ));
        let stmts = parse_ok("function g(\"OPEN\"|\"CLOSED\" state) { }");
        let Stmt::Function { params, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(&params[0].1, TypeDescriptor::Union(m) if m.len() == 2));
    }

    #[test]
    fn parses_qualified_type() {
        assert!(matches!(
            var_type("mymod:Config c = x;"),
            TypeDescriptor::Qualified { module, name } if module == "mymod" && name == "Config"
        ));
    }

    #[test]
    fn parses_distinct_type() {
        assert!(matches!(
            var_type("distinct Error e = x;"),
            TypeDescriptor::Distinct(_)
        ));
    }

    #[test]
    fn parses_object_type() {
        assert!(matches!(
            var_type("object { int x; } o = y;"),
            TypeDescriptor::Object
        ));
    }

    #[test]
    fn parses_function_type_in_return_position() {
        let stmts = parse_ok("function f() returns function (int) returns int { return g; }");
        assert!(matches!(
            &stmts[0],
            Stmt::Function {
                return_type: Some(TypeDescriptor::Function { .. }),
                ..
            }
        ));
    }

    #[test]
    fn optional_named_type_still_parses() {
        assert!(matches!(
            var_type("Person? p = x;"),
            TypeDescriptor::Optional(inner) if matches!(inner.as_ref(), TypeDescriptor::Basic(n) if n == "Person")
        ));
    }

    #[test]
    fn comparison_statement_is_not_misread_as_declaration() {
        // `a < b;` must stay an expression statement, not a generic-typed decl.
        let stmts = parse_ok("function f() { a < b; }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::Expression {
                expression: Expr::Binary {
                    op: BinaryOp::Less,
                    ..
                },
                ..
            }
        ));
    }

    #[test]
    fn named_type_var_decl_has_no_false_semantic_gap() {
        // A user-defined/predeclared type name should parse as a typed var decl.
        assert!(matches!(
            &parse_ok("json data = x;")[0],
            Stmt::VarDecl {
                type_annotation: Some(TypeDescriptor::Basic(_)),
                ..
            }
        ));
    }

    #[test]
    fn foreach_named_type_and_var_forms() {
        let stmts = parse_ok(
            "function f() { foreach Person p in people { } foreach var q in items { } foreach int i in nums { } }",
        );
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::Foreach {
                type_annotation: Some(_),
                variable,
                ..
            } if variable == "p"
        ));
        assert!(matches!(
            &body[1],
            Stmt::Foreach {
                type_annotation: None,
                variable,
                ..
            } if variable == "q"
        ));
        assert!(matches!(&body[2], Stmt::Foreach { variable, .. } if variable == "i"));
    }

    // ---- Phase 2: module-level declarations -----------------------------------

    #[test]
    fn parses_type_definition() {
        let stmts = parse_ok("public type Person record { int id; string name; };");
        assert!(matches!(
            &stmts[0],
            Stmt::TypeDef {
                is_public: true,
                name,
                descriptor: TypeDescriptor::Record { .. },
                ..
            } if name == "Person"
        ));
    }

    #[test]
    fn parses_type_alias_of_union() {
        assert!(matches!(
            &parse_ok("type Id int|string;")[0],
            Stmt::TypeDef {
                descriptor: TypeDescriptor::Union(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_enum_definition() {
        let stmts = parse_ok("public enum Color { RED, GREEN, BLUE }");
        assert!(matches!(
            &stmts[0],
            Stmt::EnumDef { is_public: true, members, .. } if members.len() == 3
        ));
    }

    #[test]
    fn parses_enum_with_explicit_values() {
        let stmts = parse_ok("enum Status { OPEN = \"open\", CLOSED = \"closed\" }");
        let Stmt::EnumDef { members, .. } = &stmts[0] else {
            panic!("expected enum")
        };
        assert_eq!(members.len(), 2);
        assert!(members.iter().all(|m| m.value.is_some()));
    }

    #[test]
    fn parses_class_with_fields_and_methods() {
        let stmts = parse_ok(
            "public class Counter { private int count = 0; public function inc() { self.count += 1; } function get() returns int { return self.count; } }",
        );
        let Stmt::ClassDef { members, .. } = &stmts[0] else {
            panic!("expected class")
        };
        // One field + two methods.
        assert_eq!(members.len(), 3);
        assert!(matches!(&members[0], Stmt::VarDecl { .. }));
        assert!(matches!(&members[1], Stmt::Function { .. }));
        assert!(matches!(&members[2], Stmt::Function { .. }));
    }

    #[test]
    fn parses_configurable_declarations() {
        assert!(matches!(
            &parse_ok("configurable int port = 8080;")[0],
            Stmt::VarDecl { name, .. } if name == "port"
        ));
        // `= ?` required configurable.
        assert!(matches!(
            &parse_ok("configurable string host = ?;")[0],
            Stmt::VarDecl {
                initializer: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_service_declaration_leniently() {
        assert!(matches!(
            &parse_ok("service /api on ln { function get() { } }")[0],
            Stmt::ServiceDecl { .. }
        ));
    }

    #[test]
    fn field_assignment_and_new_are_valid() {
        let stmts = parse_ok("function f() { self.count = 1; Counter c = new; arr[0] += 2; }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::Expression {
                expression: Expr::MemberAssign { .. },
                ..
            }
        ));
        assert!(matches!(
            &body[1],
            Stmt::VarDecl {
                initializer: Some(Expr::New { .. }),
                ..
            }
        ));
        assert!(matches!(
            &body[2],
            Stmt::Expression {
                expression: Expr::MemberAssign { .. },
                ..
            }
        ));
    }

    #[test]
    fn field_access_parses_as_field_not_variable() {
        let stmts = parse_ok("function f() { int x = obj.field; }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::VarDecl {
                initializer: Some(Expr::FieldAccess { field, .. }),
                ..
            } if field == "field"
        ));
    }

    // ---- Phase 3: error handling + core expressions ---------------------------

    /// Extracts the initializer expression from `T name = <expr>;`.
    fn var_init(src: &str) -> Expr {
        match parse_ok(src).into_iter().next().unwrap() {
            Stmt::VarDecl {
                initializer: Some(e),
                ..
            } => e,
            other => panic!("expected initialized var decl, got {:?}", other),
        }
    }

    #[test]
    fn parses_check_and_checkpanic_and_trap() {
        assert!(matches!(
            var_init("int x = check readData();"),
            Expr::Check { keyword, .. } if keyword == "check"
        ));
        assert!(matches!(
            var_init("int x = checkpanic readData();"),
            Expr::Check { keyword, .. } if keyword == "checkpanic"
        ));
        assert!(matches!(
            var_init("int x = trap readData();"),
            Expr::Check { keyword, .. } if keyword == "trap"
        ));
    }

    #[test]
    fn parses_single_and_parenthesised_arrow_functions() {
        assert!(matches!(
            var_init("var f = x => x + 1;"),
            Expr::Arrow { params, .. } if params.len() == 1
        ));
        assert!(matches!(
            var_init("var g = (a, b) => a + b;"),
            Expr::Arrow { params, .. } if params.len() == 2
        ));
        assert!(matches!(
            var_init("var h = (int a, string b) => a;"),
            Expr::Arrow { params, .. } if params == vec!["a".to_string(), "b".to_string()]
        ));
        // Zero-parameter arrow.
        assert!(matches!(
            var_init("var k = () => 42;"),
            Expr::Arrow { params, .. } if params.is_empty()
        ));
    }

    #[test]
    fn parses_anonymous_function() {
        assert!(matches!(
            var_init("var f = function(int x) returns int { return x; };"),
            Expr::AnonFunction {
                return_type: Some(_),
                params,
                ..
            } if params.len() == 1
        ));
    }

    #[test]
    fn parses_let_expression() {
        assert!(matches!(
            var_init("int x = let int a = 1, int b = 2 in a + b;"),
            Expr::Let { bindings, .. } if bindings.len() == 2
        ));
    }

    #[test]
    fn parses_typeof_expression() {
        assert!(matches!(var_init("var t = typeof x;"), Expr::TypeOf { .. }));
    }

    #[test]
    fn parses_remote_call() {
        let stmts = parse_ok("function f() { var r = client->get(\"/path\"); }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::VarDecl {
                initializer: Some(Expr::RemoteCall { method, .. }),
                ..
            } if method == "get"
        ));
    }

    #[test]
    fn subtraction_still_parses_after_arrow_token_added() {
        // Ensure `-` / `-=` still work now that `->` is lexed.
        assert!(matches!(
            var_init("int x = a - b;"),
            Expr::Binary {
                op: BinaryOp::Minus,
                ..
            }
        ));
    }

    // ---- Phase 4: statements & control flow -----------------------------------

    /// Returns the statements of the body of a single top-level function.
    fn fn_body(src: &str) -> Vec<Stmt> {
        match parse_ok(src).into_iter().next().unwrap() {
            Stmt::Function { body, .. } => body,
            other => panic!("expected function, got {:?}", other),
        }
    }

    #[test]
    fn parses_match_statement_with_alternatives_and_guard() {
        let body = fn_body(
            "function f() { match x { 1 => { } 2 | 3 => { } var y if y > 0 => { } _ => { } } }",
        );
        let Stmt::Match { arms, .. } = &body[0] else {
            panic!("expected match")
        };
        assert_eq!(arms.len(), 4);
        assert_eq!(arms[1].patterns.len(), 2); // 2 | 3
        assert!(arms[2].guard.is_some());
        assert!(matches!(arms[3].patterns[0], MatchPattern::Wildcard));
    }

    #[test]
    fn parses_list_and_mapping_match_patterns() {
        let body =
            fn_body("function f() { match x { [var a, var b] => { } {name: var n} => { } } }");
        let Stmt::Match { arms, .. } = &body[0] else {
            panic!("expected match")
        };
        assert!(matches!(&arms[0].patterns[0], MatchPattern::List(items) if items.len() == 2));
        assert!(matches!(&arms[1].patterns[0], MatchPattern::Mapping(f) if f.len() == 1));
        // Bindings collected from patterns.
        assert_eq!(arms[0].bindings, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn parses_do_on_fail() {
        let body = fn_body("function f() { do { } on fail error e { } }");
        assert!(matches!(
            &body[0],
            Stmt::DoOnFail {
                on_fail_var: Some(v),
                ..
            } if v == "e"
        ));
    }

    #[test]
    fn parses_lock_transaction_fail_and_worker() {
        let body = fn_body(
            "function f() { lock { } transaction { } fail error(\"x\"); worker w { } fork { } }",
        );
        assert!(matches!(&body[0], Stmt::Lock { .. }));
        assert!(matches!(&body[1], Stmt::Transaction { .. }));
        assert!(matches!(&body[2], Stmt::Fail { .. }));
        assert!(matches!(&body[3], Stmt::Worker { name, .. } if name == "w"));
        assert!(matches!(&body[4], Stmt::Fork { .. }));
    }

    #[test]
    fn parses_retry_transaction() {
        let body = fn_body("function f() { retry transaction { int x = 1; } }");
        assert!(matches!(&body[0], Stmt::Retry { .. }));
    }

    #[test]
    fn parses_bare_block_statement() {
        let body = fn_body("function f() { { int x = 1; } }");
        assert!(matches!(&body[0], Stmt::Block { .. }));
    }

    #[test]
    fn map_literal_still_parses_in_expression_position() {
        // A `{...}` on the RHS of `=` is still a mapping constructor, not a block.
        assert!(matches!(
            var_init("map<int> m = {a: 1};"),
            Expr::MapLiteral { .. }
        ));
    }

    // ---- Phase 5: query expressions, table constructors -----------------------

    #[test]
    fn parses_query_expression_with_clauses() {
        let query = var_init(
            "int[] r = from int n in nums where n > 2 let int t = n * 2 order by n descending limit 5 select t;",
        );
        let Expr::Query { clauses, .. } = query else {
            panic!("expected query")
        };
        assert!(
            matches!(&clauses[0], QueryClause::From { vars, .. } if vars == &vec!["n".to_string()])
        );
        assert!(clauses.iter().any(|c| matches!(c, QueryClause::Where(_))));
        assert!(clauses.iter().any(|c| matches!(c, QueryClause::Let(_))));
        assert!(clauses.iter().any(|c| matches!(c, QueryClause::OrderBy(_))));
        assert!(clauses.iter().any(|c| matches!(c, QueryClause::Limit(_))));
        assert!(matches!(clauses.last(), Some(QueryClause::Select(_))));
    }

    #[test]
    fn parses_query_with_join() {
        let query = var_init("int[] r = from int a in xs join int b in ys on a equals b select a;");
        let Expr::Query { clauses, .. } = query else {
            panic!("expected query")
        };
        assert!(clauses.iter().any(
            |c| matches!(c, QueryClause::Join { vars, .. } if vars == &vec!["b".to_string()])
        ));
    }

    #[test]
    fn parses_table_constructor() {
        assert!(matches!(
            var_init("var t = table [ {id: 1}, {id: 2} ];"),
            Expr::TableConstructor { rows, .. } if rows.len() == 2
        ));
    }

    #[test]
    fn table_as_index_target_is_not_a_constructor() {
        // `table[0]` where `table` is a variable must remain member access.
        assert!(matches!(
            var_init("int x = table[0];"),
            Expr::MemberAccess { .. }
        ));
    }

    #[test]
    fn parses_is_type_test() {
        let Expr::TypeTest { ty, .. } = var_init("boolean b = x is int;") else {
            panic!("expected type test")
        };
        assert!(matches!(ty, TypeDescriptor::Basic(n) if n == "int"));
        // `is` inside a paren-less condition (Ballerina style).
        let body = fn_body("function f() { if v is Employee { } }");
        assert!(matches!(
            &body[0],
            Stmt::If {
                condition: Expr::TypeTest { .. },
                ..
            }
        ));
    }

    #[test]
    fn from_is_usable_as_identifier_outside_query_position() {
        // A field named `from` (contextual keyword) should still be accessible.
        let stmts = parse_ok("function f() { int x = obj.from; }");
        let Stmt::Function { body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &body[0],
            Stmt::VarDecl {
                initializer: Some(Expr::FieldAccess { field, .. }),
                ..
            } if field == "from"
        ));
    }

    // ---- Real-world grammar constructs ----------------------------------------

    #[test]
    fn parses_typed_const() {
        assert!(matches!(
            &parse_ok("const int MAX = 5;")[0],
            Stmt::ConstDecl {
                type_annotation: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parses_dotted_import_with_alias() {
        assert!(matches!(
            &parse_ok("import ballerina/lang.runtime as rt;")[0],
            Stmt::Import { package_path, .. } if package_path == &vec!["ballerina".to_string(), "lang".to_string(), "runtime".to_string()]
        ));
    }

    #[test]
    fn parses_parenless_if_and_range_foreach() {
        let body = fn_body("function f() { foreach int i in 0...9 { if i > 5 { } } }");
        let Stmt::Foreach {
            iterable, body: fb, ..
        } = &body[0]
        else {
            panic!("expected foreach")
        };
        assert!(matches!(iterable, Expr::Range { .. }));
        assert!(matches!(&fb[0], Stmt::If { .. }));
    }

    #[test]
    fn parses_destructuring_declarations() {
        assert!(matches!(
            &parse_ok("function f() { var [a, b] = t; }")
                .into_iter()
                .next()
                .and_then(|s| if let Stmt::Function { body, .. } = s { body.into_iter().next() } else { None })
                .unwrap(),
            Stmt::DestructureDecl { names, .. } if names == &vec!["a".to_string(), "b".to_string()]
        ));
        // Typed mapping destructure.
        let body = fn_body("function f() { Rec {x, y} = r; }");
        assert!(matches!(&body[0], Stmt::DestructureDecl { names, .. } if names.len() == 2));
    }

    #[test]
    fn parses_optional_and_annotation_access() {
        assert!(matches!(
            var_init("string? a = user?.address;"),
            Expr::FieldAccess { field, .. } if field == "address"
        ));
        assert!(matches!(
            var_init("var a = t.@annot;"),
            Expr::FieldAccess { field, .. } if field == "annot"
        ));
    }

    #[test]
    fn parses_worker_receive_and_expression_bodied_function() {
        assert!(matches!(
            var_init("int x = <- w1;"),
            Expr::Check { keyword, .. } if keyword == "<-"
        ));
        let stmts = parse_ok("function double(int x) returns int => x * 2;");
        assert!(matches!(
            &stmts[0],
            Stmt::Function { body, .. } if matches!(body.first(), Some(Stmt::Return { .. }))
        ));
    }

    #[test]
    fn parses_quoted_identifier_and_builtin_subtype() {
        // `'int` is a quoted identifier; `int:Signed32` is a subtype.
        assert!(matches!(
            &parse_ok("int 'version = 1;")[0],
            Stmt::VarDecl { name, .. } if name == "'version"
        ));
        assert!(matches!(
            var_type("int:Signed32 x = y;"),
            TypeDescriptor::Qualified { module, name } if module == "int" && name == "Signed32"
        ));
    }

    #[test]
    fn parses_strict_equality_operators() {
        assert!(matches!(
            var_init("boolean b = a === c;"),
            Expr::Binary {
                op: BinaryOp::EqualEqualEqual,
                ..
            }
        ));
        assert!(matches!(
            var_init("boolean b = a !== c;"),
            Expr::Binary {
                op: BinaryOp::NotEqualEqual,
                ..
            }
        ));
    }

    #[test]
    fn parses_hex_and_suffixed_numbers() {
        assert!(matches!(
            var_init("int n = 0xFF;"),
            Expr::Literal {
                value: Literal::Number(n),
                ..
            } if n == 255.0
        ));
        assert!(matches!(
            var_init("decimal d = 12.5d;"),
            Expr::Literal { .. }
        ));
    }

    #[test]
    fn parses_query_do_action_statement() {
        let body = fn_body("function f() { from int i in xs do { g(i); }; }");
        assert!(matches!(
            &body[0],
            Stmt::Expression {
                expression: Expr::Query { .. },
                ..
            }
        ));
    }

    #[test]
    fn parses_destructuring_foreach() {
        let body = fn_body("function f() { foreach [string, int] [name, grade] in rows { } }");
        assert!(matches!(
            &body[0],
            Stmt::Foreach { variable, extra_bindings, .. }
                if variable == "name" && extra_bindings == &vec!["grade".to_string()]
        ));
    }

    #[test]
    fn ternary_branch_does_not_swallow_qualified_colon() {
        // `m is string ? m : log:eval(m)` — the `:` ends the true branch.
        assert!(matches!(
            var_init("string s = m is string ? m : log:eval(m);"),
            Expr::Ternary { .. }
        ));
    }

    #[test]
    fn parses_keyword_qualified_references() {
        // `transaction:Info` as a type and `transaction:onCommit(..)` as a call.
        let stmts = parse_ok("function f(transaction:Info info) { transaction:onCommit(h); }");
        let Stmt::Function { params, body, .. } = &stmts[0] else {
            panic!("expected function")
        };
        assert!(matches!(
            &params[0].1,
            TypeDescriptor::Qualified { module, .. } if module == "transaction"
        ));
        assert!(matches!(
            &body[0],
            Stmt::Expression {
                expression: Expr::Call { .. },
                ..
            }
        ));
    }

    #[test]
    fn parses_xml_step_and_multi_key_access() {
        assert!(matches!(
            var_init("xml items = doc.<items>;"),
            Expr::FieldAccess { field, .. } if field == "items"
        ));
        assert!(matches!(
            var_init("var e = employees[\"John\", \"Bloggs\"];"),
            Expr::MemberAccess { .. }
        ));
    }

    #[test]
    fn parses_mapping_shorthand_and_spread() {
        let e = var_init("var m = {name, age: 30, ...rest};");
        let Expr::MapLiteral { entries, .. } = e else {
            panic!("expected map literal")
        };
        assert_eq!(entries.len(), 3);
        // Shorthand `name` becomes `name: name`.
        assert!(matches!(&entries[0].1, Expr::Variable { name, .. } if name == "name"));
    }
}
