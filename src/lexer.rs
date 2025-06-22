use crate::token::{Token, TokenType};

pub struct Lexer {
  source: String,
  chars: Vec<char>,
  start: usize,
  current: usize,
  line: usize,
}

impl Lexer {
  pub fn new(source: String) -> Self {
    let chars: Vec<char> = source.chars().collect();
    Self {
      source,
      chars,
      start: 0,
      current: 0,
      line: 0,
    }
  }

  pub fn scan_tokens(&mut self) -> Vec<Token> {
    let mut tokens = Vec::new();

    while !self.is_at_end() {
      self.start = self.current;
      if let Some(token) = self.scan_token() {
        tokens.push(token);
      }
    }

    tokens.push(Token {
      token_type: TokenType::Eof,
      lexeme: "".to_string(),
      line: self.line,
    });

    tokens
  }

  /// Scans and returns the next valid token, or `None` to skip whitespace/comments.
  /// This is the main driver function for lexing.
  fn scan_token(&mut self) -> Option<Token> {
    let ch = self.advance();

    match ch {
      /// Create single char tokens
      '(' => Some(self.make_token(TokenType::LeftParen)),
      ')' => Some(self.make_token(TokenType::RightParen)),
      '{' => Some(self.make_token(TokenType::LeftBrace)),
      '}' => Some(self.make_token(TokenType::RightBrace)),
      ',' => Some(self.make_token(TokenType::Comma)),
      '.' => Some(self.make_token(TokenType::Dot)),
      '-' => Some(self.make_token(TokenType::Minus)),
      '+' => Some(self.make_token(TokenType::Plus)),
      ';' => Some(self.make_token(TokenType::SemiColon)),
      '*' => Some(self.make_token(TokenType::Star)),

      /// Create double char tokens
      '=' => {
        if self.match_char('=') {
          Some(self.make_token(TokenType::EqualEqual))
        } else {
          Some(self.make_token(TokenType::Equal))
        }
      }

      '!' => {
        if self.match_char('=') {
          Some(self.make_token(TokenType::BangEqual))
        } else {
          Some(self.make_token(TokenType::Bang))
        }
      }

      '<' => {
        if self.match_char('=') {
          Some(self.make_token(TokenType::LessEqual))
        } else {
          Some(self.make_token(TokenType::Less))
        }
      }

      '>' => {
        if self.match_char('=') {
          Some(self.make_token(TokenType::GreaterEqual))
        } else {
          Some(self.make_token(TokenType::Greater))
        }
      }

      // Create number tokens
      '0'..='9' => self.number(),

      // Ignore spaces, carriage returns, tabs and newlines
      ' ' | '\r' | '\t' => None,
      '\n' => {
        self.line += 1;
        None
      }

      _ => {
        // Later: handle indentifiers, numbers, keywords
        println!("Unexpected charcter: {}", ch);
        None
      }
    }
  }

  /// Create a new `Token` from the current lexeme span.
  /// The span is from `start` to `current`
  fn make_token(&self, token_type: TokenType) -> Token {
    let text: String = self.chars[self.start..self.current].iter().collect();
    Token {
      token_type,
      lexeme: text,
      line: self.line,
    }
  }

  //----------------------------- Helper Methods ----------------------------

  /// Return `true` if the lexer has reached the end of the source string.
  fn is_at_end(&self) -> bool {
    self.current >= self.chars.len()
  }

  /// Comsumes and return the next char in the source.
  /// Advance the 'current' pointer forward.
  /// Returns a 'char' not a byte.
  fn advance(&mut self) -> char {
    let ch = self.chars[self.current];
    self.current += 1;
    ch
  }

  /// Conditionally consumes the next char if it matches `expected`
  /// Returns `true` if the match succeeded and advanced the cursor.
  /// Otherwise, returns `false` and does not advance.
  fn match_char(&mut self, expected: char) -> bool {
    if self.is_at_end() {
      return false;
    }
    if self.source[self.current..].chars().next().unwrap() != expected {
      return false;
    }

    self.current += expected.len_utf8();
    true
  }

  /// Scans a number literal: integers or floats
  fn number(&mut self) -> Option<Token> {
    while let Some(c) = self.peek() {
      if c.is_ascii_digit() {
        self.advance();
      } else {
        break;
      }
    }

    // Check for a fractional part
    if self.peek() == Some('.')
      && self
        .peek_next()
        .map(|c| c.is_ascii_digit())
        .unwrap_or(false)
    {
      self.advance();
      while let Some(c) = self.peek() {
        if c.is_ascii_digit() {
          self.advance();
        } else {
          break;
        }
      }
    }

    Some(self.make_token(TokenType::Number))
  }

  /// Peeks at the current char without advancing
  /// Returns `None` if at end of input
  fn peek(&self) -> Option<char> {
    self.source[self.current..].chars().next()
  }

  /// Peeks at the char after the current one without advancing
  fn peek_next(&self) -> Option<char> {
    let mut chars = self.source[self.current..].chars();
    chars.next(); // skip current
    chars.next() // return next
  }
}
