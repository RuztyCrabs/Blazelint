#[derive(Debug, PartialEq, Clone)]
pub enum Token {
  /// Keywords
  Var,
  Function,
  If,
  Else,
  While,
  Foreach,
  In,
  Return,
  Panic,
  Check,
  Returns, // For function return types
  Int,
  String,
  Boolean,
  Float,
  True,
  False,

  // Operators
  Plus,
  Minus,
  Star,
  Slash,
  Bang,
  Eq,
  EqEq,
  BangEq,
  Gt,
  Ge,
  Lt,
  Le,
  AmpAmp,
  PipePipe,

  // Delimiters
  LParen,
  RParen,
  LBrace,
  RBrace,
  Colon,
  Semicolon,
  Comma,

  // Literals
  Number(f64),
  StringLiteral(String),
  Identifier(String),
}

pub struct Lexer<'input> {
  input: &'input str,
  chars: std::iter::Peekable<std::str::Chars<'input>>,
  start: usize,   // Start offset of the current lexeme
  current: usize, // Current char being considered
}

impl<'input> Lexer<'input> {
  pub fn new(input: &'input str) -> Self {
    Lexer {
      input,
      chars: input.chars().peekable(),
      start: 0,
      current: 0,
    }
  }

  /// Advances the `current` pointer and consumes the next char
  fn advance(&mut self) -> Option<char> {
    let c = self.chars.next();
    if let Some(ch) = c {
      self.current += ch.len_utf8();
    }
    c
  }

  /// Peeks at the next char without consuming it
  fn peek(&mut self) -> Option<&char> {
    self.chars.peek()
  }

  /// Peeks at the char two positons ahead without consuming it
  fn peek_next(&mut self) -> Option<char> {
    let mut temp_chars = self.chars.clone();
    temp_chars.next(); // Consume the first char
    temp_chars.next() // Peek at the second
  }

  /// Checks if the next char matches `expected` and  consumes it if so
  fn match_char(&mut self, expected: char) -> bool {
    if let Some(&c) = self.peek() {
      if c == expected {
        self.advance();
        true
      } else {
        false
      }
    } else {
      false
    }
  }

  /// Checks if the current cursor is a tthe end of the input
  fn is_at_end(&mut self) -> bool {
    self.peek().is_none()
  }

  /// Create a token from the `start` to `current` position
  fn create_token(&self, token_type: Token) -> (usize, Token, usize) {
    (self.start, token_type, self.current)
  }

  /// Skips whitespace and comments.
  fn skip_whitespace_and_comments(&mut self) {
    loop {
      if self.is_at_end() {
        return;
      }
      let c = *self.peek().unwrap();

      match c {
        ' ' | '\r' | '\t' | '\n' => {
          self.advance();
        }
        '/' => {
          // Check for comments
          if self.peek_next() == Some('/') {
            // Single-line comment //
            self.advance(); // Consume '/'
            self.advance(); // consumes sencond '/'
            while self.peek() != Some(&'\n') && !self.is_at_end() {
              self.advance(); // Consume chars until newline or EOF
            }
            self.advance(); // Consume the newline (if present)
          } else if self.peek_next() == Some('*') {
            // Multi-line comment /* ... */
            self.advance(); // comsume '/'
            self.advance(); // consume '*'
            let mut found_end_comment = false;
            while !self.is_at_end() {
              if self.peek() == Some(&'*') && self.peek_next() == Some('/') {
                self.advance(); // Consume '*'
                self.advance(); // Consume '/''
                found_end_comment = true;
                break;
              }
              self.advance();
            }
            if !found_end_comment {
              // reserved for report an error later
              // for now, just breaks the lexer
              return;
            }
          } else {
            break; // Not a comment, break the loop to process '/' as an operator
          }
        }
        _ => break, // Not a whitespace of comment, exit loop
      }
    }
  }

  // Scan a string literal
  fn string(&mut self) -> Result<Token, String> {
    while self.peek() != Some(&'"') && !self.is_at_end() {
      if self.peek() == Some(&'\\') {
        self.advance();
        if self.is_at_end() {
          return Err(format!("Unterminated escape sequence at {}", self.start));
        }
      }
      self.advance();
    }

    if self.is_at_end() {
      return Err(format!("Unterminated string at {}", self.start));
    }
    self.advance(); // Consume the closing '""'

    // Extract the string value (exclude surrounding quotes)
    let value = self.input[self.start + 1..self.current - 1].to_string();

    // Simple unescape for \"
    let unescaped_value = value.replace("\\\"", "\"");
    Ok(Token::StringLiteral(unescaped_value))
  }

  /// Scans a number literal (integer or float)
  fn number(&mut self) -> Result<Token, String> {
    while self.peek().map_or(false, |&c| c.is_ascii_digit()) {
      self.advance();
    }

    // Look for a fractional part
    if self.peek() == Some(&'.') && self.peek_next().map_or(false, |c| c.is_ascii_digit()) {
      self.advance(); // Consume '.'
      while self.peek().map_or(false, |&c| c.is_ascii_digit()) {
        self.advance();
      }
    }

    // Look for exponent part
    if self.peek().map_or(false, |&c| c == 'e' || c == 'E') {
      self.advance(); // Consume 'e' or 'E'
      if self.peek().map_or(false, |&c| c == '+' || c == '-') {
        self.advance(); // Consume '+'  or '-'
      }
      if self.peek().map_or(false, |&c| c.is_ascii_digit()) {
        while self.peek().map_or(false, |&c| c.is_ascii_digit()) {
          self.advance();
        }
      } else {
        return Err(format!(
          "Malformed exponent in number at byte offset{}",
          self.start
        ));
      }
    }

    let value_str = &self.input[self.start..self.current];
    value_str
      .parse::<f64>()
      .map(Token::Number)
      .map_err(|e| format!("Invalid number literal '{}': {}", value_str, e))
  }

  /// Scan an Idnentifier or Keyword
  fn identifier(&mut self) -> Token {
    while self
      .peek()
      .map_or(false, |&c| c.is_ascii_alphanumeric() || c == '_')
    {
      self.advance();
    }

    let text = &self.input[self.start..self.current];
    match text {
      "var" => Token::Var,
      "function" => Token::Function,
      "if" => Token::If,
      "else" => Token::Else,
      "while" => Token::While,
      "foreach" => Token::Foreach,
      "in" => Token::In,
      "return" => Token::Return,
      "panic" => Token::Panic,
      "check" => Token::Check,
      "returns" => Token::Returns,
      "int" => Token::Int,
      "string" => Token::String,
      "boolean" => Token::Boolean,
      "float" => Token::Float,
      "true" => Token::True,
      "false" => Token::False,
      _ => Token::Identifier(text.to_string()),
    }
  }
}

/// Implement the iterator trait for the lexer
impl<'input> Iterator for Lexer<'input> {
  type Item = Result<(usize, Token, usize), String>;

  fn next(&mut self) -> Option<Self::Item> {
    // Skip whitespace and comments before finding the next token
    self.skip_whitespace_and_comments();

    // Update start position for the new token after skipping
    self.start = self.current;

    // Check for end of input AFTER skipping
    let c = self.advance()?; // Try to advance and get the first char of the next token
                             // If None, it means we're at the end of the file

    let result = match c {
      '(' => Ok(self.create_token(Token::LParen)),
      ')' => Ok(self.create_token(Token::RParen)),
      '{' => Ok(self.create_token(Token::LBrace)),
      '}' => Ok(self.create_token(Token::RBrace)),
      ':' => Ok(self.create_token(Token::Colon)),
      ';' => Ok(self.create_token(Token::Semicolon)),
      ',' => Ok(self.create_token(Token::Comma)),
      '+' => Ok(self.create_token(Token::Plus)),
      '-' => Ok(self.create_token(Token::Minus)),
      '*' => Ok(self.create_token(Token::Star)),
      '/' => Ok(self.create_token(Token::Slash)),
      '!' => {
        if self.match_char('=') {
          Ok(self.create_token(Token::BangEq))
        } else {
          Ok(self.create_token(Token::Bang))
        }
      }
      '=' => {
        if self.match_char('=') {
          Ok(self.create_token(Token::EqEq))
        } else {
          Ok(self.create_token(Token::Eq))
        }
      }
      '>' => {
        if self.match_char('=') {
          Ok(self.create_token(Token::Ge))
        } else {
          Ok(self.create_token(Token::Gt))
        }
      }
      '<' => {
        if self.match_char('=') {
          Ok(self.create_token(Token::Le))
        } else {
          Ok(self.create_token(Token::Lt))
        }
      }
      '&' => {
        if self.match_char('&') {
          Ok(self.create_token(Token::AmpAmp))
        } else {
          Err(format!(
            "Unexpected character: '&' at byte offset {}",
            self.start
          ))
        }
      }
      '|' => {
        if self.match_char('|') {
          Ok(self.create_token(Token::PipePipe))
        } else {
          Err(format!(
            "Unexpected character: '|' at byte offset {}",
            self.start
          ))
        }
      }
      '"' => self.string().map(|t| self.create_token(t)), // Scan string literal
      d if d.is_ascii_digit() => self.number().map(|t| self.create_token(t)), // Scan number literal
      a if a.is_ascii_alphabetic() || a == '_' =>  {
        // Call the mutable method first
        let id_token = self.identifier();
        // Now that the mutable borrow from `self.identifier()` is released
        Ok(self.create_token(id_token))
      },
      _ => Err(format!(
        "Unexpected charcter: '{}' at byte offset {}",
        c, self.start
      )),
    };

    Some(result)
  }
}
