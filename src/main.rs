mod lexer;
mod token;
mod parser;
mod ast;
mod linter;
mod error;

use token::{Token, TokenType};
use lexer::Lexer;

fn main() {
    println!("Welcome to BlazeLint! ");

    let source = "() { } , . - + * ;".to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.scan_tokens();

    for token in tokens {
        println!("{:?}", token);
    }

//    let example_token = Token {
//        token_type: TokenType::Var,
//        lexeme: "Var".to_string(),
//        line: 1,
//    };
//
//    println!("{:?}", example_token);
}
