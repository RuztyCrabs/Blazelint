//! Tokeniser for the Blazelint front-end.
//!
//! The lexer converts raw source text into a stream of token triples annotated
//! with byte offsets. Subsequent stages use these spans to highlight precise
//! error locations and to reconstruct lexemes as needed.
use crate::errors::LexError;

/// Tokens recognised by the Ballerina subset Blazelint currently supports.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// Keywords
    Final,
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
    Returns,
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

/// Streaming lexer that yields `(start, token, end)` triples for each lexeme.
pub struct Lexer<'input> {
    /// Entire source being tokenised.
    input: &'input str,
    /// Iterator used to peek and consume characters.
    chars: std::iter::Peekable<std::str::Chars<'input>>,
    /// Start byte offset of the current lexeme.
    start: usize,
    /// Cursor pointing at the next character to process.
    current: usize,
}

impl<'input> Lexer<'input> {
    /// Creates a lexer positioned at the start of `input`.
    pub fn new(input: &'input str) -> Self {
        Lexer {
            input,
            chars: input.chars().peekable(),
            start: 0,
            current: 0,
        }
    }

    /// Skips whitespace and comments, reporting unterminated block comments as errors.
    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            if self.is_at_end() {
                return Ok(());
            }

            let c = match self.peek() {
                Some(ch) => *ch,
                None => return Ok(()),
            };

            match c {
                ' ' | '\r' | '\t' | '\n' => {
                    self.advance();
                }
                '/' => {
                    let comment_start = self.current;
                    if self.peek_next() == Some('/') {
                        // Single-line comment //
                        self.advance(); // Consume '/'
                        self.advance(); // Consume second '/'
                        while self.peek() != Some(&'\n') && !self.is_at_end() {
                            self.advance();
                        }
                        if self.peek() == Some(&'\n') {
                            self.advance();
                        }
                    } else if self.peek_next() == Some('*') {
                        // Multi-line comment /* ... */
                        self.advance(); // consume '/'
                        self.advance(); // consume '*'
                        let mut found_end_comment = false;
                        while !self.is_at_end() {
                            if self.peek() == Some(&'*') && self.peek_next() == Some('/') {
                                self.advance(); // Consume '*'
                                self.advance(); // Consume '/'
                                found_end_comment = true;
                                break;
                            }
                            self.advance();
                        }
                        if !found_end_comment {
                            return Err(LexError::new(
                                "Unterminated block comment",
                                comment_start..self.current,
                            ));
                        }
                    } else {
                        return Ok(());
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Scans a string literal, producing a `LexError` for unterminated strings or escapes.
    fn string(&mut self) -> Result<Token, LexError> {
        while self.peek() != Some(&'"') && !self.is_at_end() {
            if self.peek() == Some(&'\\') {
                self.advance();
                if self.is_at_end() {
                    return Err(LexError::new(
                        format!("Unterminated escape sequence"),
                        self.start..self.current,
                    ));
                }
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(LexError::new(
                "Unterminated string literal",
                self.start..self.current,
            ));
        }
        self.advance(); // Consume the closing '""'

        // Extract the string value (exclude surrounding quotes)
        let value = self.input[self.start + 1..self.current - 1].to_string();

        // Simple unescape for \"
        let unescaped_value = value.replace("\\\"", "\"");
        Ok(Token::StringLiteral(unescaped_value))
    }

    /// Scans a numeric literal (integer, float, or float with exponent) into a token.
    fn number(&mut self) -> Result<Token, LexError> {
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
                return Err(LexError::new(
                    "Malformed exponent in number literal",
                    self.start..self.current,
                ));
            }
        }

        let value_str = &self.input[self.start..self.current];
        value_str.parse::<f64>().map(Token::Number).map_err(|e| {
            LexError::new(
                format!("Invalid number literal '{value_str}': {e}"),
                self.start..self.current,
            )
        })
    }

    /// Scans an identifier or recognises a reserved keyword in the Ballerina subset.
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
            "final" => Token::Final,
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

    //-------------- Helpers ---------------------------

    /// Creates a token triple `[start, token, end)` covering the current lexeme.
    fn create_token(&self, token_type: Token) -> (usize, Token, usize) {
        (self.start, token_type, self.current)
    }

    /// Advances the lexer and consumes the next character, if any.
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            self.current += ch.len_utf8();
        }
        c
    }

    /// Peeks at the next character without consuming it.
    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Peeks two characters ahead without moving the cursor.
    fn peek_next(&mut self) -> Option<char> {
        let mut temp_chars = self.chars.clone();
        temp_chars.next(); // Consume the first char
        temp_chars.next() // Peek at the second
    }

    /// Consumes the next character only when it matches `expected`.
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

    /// Returns true once the cursor has consumed the entire input.
    fn is_at_end(&mut self) -> bool {
        self.peek().is_none()
    }
}

/// Implements `Iterator` so the lexer can be used directly in `for` loops.
impl<'input> Iterator for Lexer<'input> {
    type Item = Result<(usize, Token, usize), LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Skip whitespace and comments before finding the next token
        if let Err(err) = self.skip_whitespace_and_comments() {
            return Some(Err(err));
        }

        // Update start position for the new token after skipping
        self.start = self.current;

        // Check for end of input AFTER skipping
        let c = match self.advance() {
            Some(ch) => ch,
            None => return None, // End of file
        };

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
                    Err(LexError::new(
                        "Unexpected single '&' character",
                        self.start..self.current,
                    ))
                }
            }
            '|' => {
                if self.match_char('|') {
                    Ok(self.create_token(Token::PipePipe))
                } else {
                    Err(LexError::new(
                        "Unexpected single '|' character",
                        self.start..self.current,
                    ))
                }
            }
            '"' => self.string().map(|t| self.create_token(t)), // Scan string literal
            d if d.is_ascii_digit() => self.number().map(|t| self.create_token(t)), // Scan number literal
            a if a.is_ascii_alphabetic() || a == '_' => {
                // Call the mutable method first
                let id_token = self.identifier();
                // Now that the mutable borrow from `self.identifier()` is released
                Ok(self.create_token(id_token))
            }
            _ => Err(LexError::new(
                format!("Unexpected character: '{c}'"),
                self.start..self.current,
            )),
        };

        Some(result)
    }
}
