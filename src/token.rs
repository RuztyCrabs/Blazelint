use std::string::String;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Single-character tokens
    LeftParen, RightParen,
    LeftBrace, RightBrace,
    Comma, Dot, SemiColon, Colon,
    Plus, Minus, Star, Slash,
    Equal, Bang,

    // One or two character tokens
    EqualEqual, BangEqual,
    Greater, GreaterEqual,
    Less, LessEqual,

    // Literals
    Identifier,
    String,
    Number,

    // Keywords
    Var,
    Function,
    If, Else,
    While,
    Foreach, In,
    Return,
    Panic,
    Check,
    True, False,

    // Types
    Int, Float, Boolean,

    // End of File
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub line: usize,
}

// Expose the module
pub use TokenType::*;
