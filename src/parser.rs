use crate::ast::*;
use crate::lexer::Token;

pub struct Parser {
  tokens: Vec<(usize, Token, usize)>,
  current: usize,
}

impl Parser {
  pub fn new(tokens: Vec<(usize, Token, usize)>) -> Self {
    Self {
      tokens,
      current: 0,
    }
  }

  pub fn parse(&mut self) -> Vec<Stmt> {
    let mut statements = Vec::new();
    while !self.is_at_end() {
      statements.push(self.declaration());
    }
    statements
  }

  fn declaration(&mut self) -> Stmt {
    match self.peek() {
      Token::Var => self.var_decl(),
      Token::Function => self.function(),
      _ => self.statement(),
    }
  }

  fn var_decl(&mut self) -> Stmt {
    self.advance(); // Consume 'Var'
    let name = if let Token::Identifier(name) = self.advance() {
      name.clone()
    } else {
      panic!("Expected variable name");
    };

    let type_annotation = if self.match_token(&[Token::Colon]) {
      Some(self.parse_type())
    } else {
      None
    };

    let initializer = if self.match_token(&[Token::Eq]) {
      Some(self.expression())
    } else {
      None
    };

    self.consume(Token::Semicolon, "Expected ';' after variable declaration");

    Stmt::VarDecl {
      name,
      type_annotation,
      initializer,
    }
  }

  fn statement(&mut self) -> Stmt {
    match self.peek() {
      Token::If => self.if_statement(),
      Token::Return => {
        self.advance();
        let expr = if self.check(&Token::Semicolon) {
          None
        } else {
          Some(self.expression())
        };
        self.consume(Token::Semicolon, "Expected ';' after");
        Stmt::Return(expr)
      }
      Token::Panic => {
        self.advance();
        let expr = self.expression();
        self.consume(Token::Semicolon, "Expected ';' after panic");
        Stmt::Panic(expr)
      }
      _ => {
        let expr = self.expression();
        self.consume(Token::Semicolon, "Expected ';'");
        Stmt::Expression(expr)
      }
    }
  }

  fn if_statement(&mut self) -> Stmt {
    self.advance(); // consume 'if'
    self.consume(Token::LParen, "Expected '(' after if");
    let condition = self.expression();
    self.consume(Token::RParen, "Expected ')' after condition");

    self.consume(Token::LBrace, "Expected '{' before then block");
    let then_block = self.block();
    let else_block = if self.match_token(&[Token::Else]) {
      self.consume(Token::LBrace, "Expected '{' before else block");
      Some(self.block())
    } else {
      None
    };

    Stmt::If {
      condition,
      then_branch: then_block,
      else_branch: else_block,
    }
  }

  fn block(&mut self) -> Vec<Stmt> {
    let mut stmts = Vec::new();
    while !self.check(&Token::RBrace) && !self.is_at_end() {
      stmts.push(self.declaration());
    }
    self.consume(Token::RBrace, "Expected '}'");
    stmts
  }

  fn function(&mut self) -> Stmt {
    self.advance(); // consume 'function'
    let name = if let Token::Identifier(n) = self.advance() {
      n.clone()
    } else {
      panic!("Expected function name");
    };

    self.consume(Token::LParen, "Expected '(' after function name");
    let mut params = Vec::new();
    while !self.check(&Token::RParen) {
      let param_name = if let Token::Identifier(name) = self.advance() {
        name.clone()
      } else {
        panic!("Expected parameter name");
      };
      self.consume(Token::Colon, "Expected ':' in parameter");
      let param_type = self.parse_type();
      params.push((param_name, param_type));
      if !self.check(&Token::RParen) {
        self.consume(Token::Comma, "Expected ',' between parameters");
      }
    }
    self.consume(Token::RParen, "Expected ')' after parameters");

    let return_type = if self.match_token(&[Token::Returns]) {
      Some(self.parse_type())
    } else {
      None
    };

    self.consume(Token::LBrace, "Expected '{' before function body");
    let body = self.block();

    Stmt::Function {
      name,
      params,
      return_type,
      body: body,
    }
  }
  fn expression(&mut self) -> Expr {
    self.assignment()
  }

  fn assignment(&mut self) -> Expr {
    let expr = self.logic_or();

    if self.match_token(&[Token::Eq]) {
      let value = self.assignment();

      if let Expr::Variable(name) = expr {
        return Expr::Assign { name, value: Box::new(value) };
      }

      panic!("Invalid assignment target");
    }

    expr
  }

  fn logic_or(&mut self) -> Expr {
    let mut expr = self.logic_and();

    while self.match_token(&[Token::PipePipe]) {
      let op = BinaryOp::Or;
      let right = self.logic_and();
      expr = Expr::Binary(Box::new(expr), op, Box::new(right));
    }

    expr
  }

  fn logic_and(&mut self) -> Expr {
    let mut expr = self.equality();

    while self.match_token(&[Token::AmpAmp]) {
      let op = BinaryOp::And;
      let right = self.equality();
      expr = Expr::Binary(Box::new(expr), op, Box::new(right));
    }

    expr
  }

  fn equality(&mut self) -> Expr {
    let mut expr = self.comparison();

    while self.match_token(&[Token::EqEq, Token::BangEq]) {
      let op = match self.previous() {
        Token::EqEq => BinaryOp::EqualEqual,
        Token::BangEq => BinaryOp::NotEqual,
        _ => unreachable!(),
      };
      let right = self.comparison();
      expr = Expr::Binary(Box::new(expr), op, Box::new(right));
    }

    expr
  }

  fn comparison(&mut self) -> Expr {
    let mut expr = self.term();

    while self.match_token(&[
      Token::Gt,
      Token::Ge,
      Token::Lt,
      Token::Le,
    ]) {
      let op = match self.previous() {
        Token::Gt => BinaryOp::Greater,
        Token::Ge => BinaryOp::GreaterEqual,
        Token::Lt => BinaryOp::Less,
        Token::Le => BinaryOp::LessEqual,
        _ => unreachable!(),
      };
      let right = self.term();
      expr = Expr::Binary(Box::new(expr), op, Box::new(right));
    }

    expr
  }

  fn term(&mut self) -> Expr {
    let mut expr = self.factor();

    while self.match_token(&[Token::Plus, Token::Minus]) {
      let op = match self.previous() {
        Token::Plus => BinaryOp::Plus,
        Token::Minus => BinaryOp::Minus,
        _ => unreachable!(),
      };
      let right = self.factor();
      expr = Expr::Binary(Box::new(expr), op, Box::new(right));
    }

    expr
  }

  fn factor(&mut self) -> Expr {
    let mut expr = self.unary();

    while self.match_token(&[Token::Star, Token::Slash]) {
      let op = match self.previous() {
        Token::Star => BinaryOp::Star,
        Token::Slash => BinaryOp::Slash,
        _ => unreachable!(),
      };
      let right = self.unary();
      expr = Expr::Binary(Box::new(expr), op, Box::new(right));
    }

    expr
  }

  fn unary(&mut self) -> Expr {
    if self.match_token(&[Token::Bang, Token::Minus]) {
      let op = match self.previous() {
        Token::Bang => UnaryOp::Bang,
        Token::Minus => UnaryOp::Minus,
        _ => unreachable!(),
      };
      let right = self.unary();
      return Expr::Unary(op, Box::new(right));
    }

    self.primary()
  }

  fn primary(&mut self) -> Expr {
    match self.advance() {
      Token::True => Expr::Literal(Literal::Boolean(true)),
      Token::False => Expr::Literal(Literal::Boolean(false)),
      Token::Number(n) => Expr::Literal(Literal::Number(*n)),
      Token::StringLiteral(s) => Expr::Literal(Literal::String(s.clone())),
      Token::Identifier(name) => Expr::Variable(name.clone()),
      Token::LParen => {
        let expr = self.expression();
        self.consume(Token::RParen, "Expected ')' after expression");
        Expr::Grouping(Box::new(expr))
      }
      t => panic!("Unexpected token in expression: {:?}", t),
    }
  }

  fn parse_type(&mut self) -> String {
    match self.advance() {
      Token::Identifier(s) => s.clone(),
      Token::Int => "int".to_string(),
      Token::String => "string".to_string(),
      Token::Boolean => "boolean".to_string(),
      Token::Float => "float".to_string(),
      t => panic!("Expected type, found {:?}", t),
    }
  }

  //-------------- Helpers ---------------------------

  fn is_at_end(&self) -> bool {
    self.current >= self.tokens.len()
  }

  fn peek(&self) -> &Token {
    &self.tokens[self.current].1
  }

  fn previous(&self) -> &Token {
    &self.tokens[self.current - 1].1
  }

  fn advance(&mut self) -> &Token {
    if !self.is_at_end() {
      self.current += 1;
    }
    self.previous()
  }

  fn check(&self, expected: &Token) -> bool {
    !self.is_at_end() && self.peek() == expected
  }

  fn match_token(&mut self, types: &[Token]) -> bool {
    for t in types {
      if self.peek() == t {
        self.advance();
        return true;
      }
    }
    false
  }

  fn consume(&mut self, expected: Token, msg: &str) {
    if self.peek() == &expected {
      self.advance();
    } else {
      panic!("{}", msg);
    }
  }
}