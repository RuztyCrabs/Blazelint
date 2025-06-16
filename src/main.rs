mod lexer;
mod token;
mod parser;
mod ast;
mod linter;
mod error;

use token::{Token, TokenType};

fn main() {
    println!("Welcome to BlazeLint! ");

    let example_token = Token {
        token_type: TokenType::Var,
        lexeme: "Var".to_string(),
        line: 1,
    };

    println!("{:?}", example_token);
}
