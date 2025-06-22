mod ast;
mod error;
mod lexer;
mod linter;
mod parser;
mod token;

use lexer::Lexer;
use token::{Token, TokenType};

fn main() {
  println!("Welcome to BlazeLint! ");

  // hard codeded a dummy bal syntax for testing the lexer
  let source = "() { } , . - + * ; == != >= <= 123 1.23 12.".to_string();
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
