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

    fn is_at_end(&self) -> bool {
        self.current >= self.chars.len()
    }

    fn advance(&mut self) -> char {
        let ch = self.chars[self.current];
        self.current += 1;
        ch
    }

    // For now: placeholder. logic is yet to be added
    fn scan_token(&mut self) -> Option<Token> {
        let ch = self.advance();

        match ch {
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

    fn make_token(&self, token_type: TokenType) -> Token {
        let text: String = self.chars[self.start..self.current].iter().collect();
        Token {
            token_type,
            lexeme: text,
            line: self.line,
        }
    }
}
