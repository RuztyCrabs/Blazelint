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
            if let Some(Token) = self.scan_token() {
                tokens.push(token);
            }
        }

        tokens.push(Token {
            token_type: TokenType::Eof,
            lexeme: "".to_string(),
            line.self.line,
        });

        tokens
    }

    fn is_at_end(&self) -> bool {
        let ch = self.chars[self.current];
        self.current += 1;
    }
}
