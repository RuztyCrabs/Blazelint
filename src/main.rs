mod ast;
mod errors;
mod lexer;
mod parser;

use lexer::Lexer;
use parser::Parser;
use std::env;
use std::fs;
use std::process;

/// Main function of the Blazelint application.
///
/// This function initializes the lexer and parser, processes the input code,
/// and prints the generated tokens and Abstract Syntax Tree (AST).
fn main() {
  println!("Ballerina Linter (WIP)");

  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    eprintln!("Usage: {} <file_path>", args[0]);
    process::exit(1);
  }

  let file_path = &args[1];
  let input_code = match fs::read_to_string(file_path) {
    Ok(code) => code,
    Err(err) => {
      eprintln!("Error reading file {}: {}", file_path, err);
      process::exit(1);
    }
  };

  println!("--- Input Code ---");
  println!("{}", input_code);
  println!("----------------------------\n");

  // Initialize the lexer with the input code and collect tokens.
  let lexer = Lexer::new(&input_code);
  let tokens: Vec<_> = lexer.collect::<Result<_, _>>().unwrap();
  // Initialize the parser with the collected tokens.
  let mut parser = Parser::new(tokens.clone());

  println!("--- Tokens ---");
  // Print each token for debugging purposes.
  for (start, token, end) in tokens {
    println!("Token: {:?} ({}..{})", token, start, end);
  }
  println!("----------------------------\n");
  println!("Lexing complete!");

  println!("-- AST --");
  // Parse the tokens to generate the Abstract Syntax Tree (AST).
  let ast = match parser.parse() {
    Ok(ast) => ast,
    Err(err) => {
      eprintln!("Parse error: {}", err.message);
      if let Some(expected) = err.expected {
        eprintln!("Expected: {}", expected);
      }
      eprintln!("Span: {}..{}", err.span.start, err.span.end);
      process::exit(1);
    }
  };
  // Print the AST in a pretty-formatted way.
  for stmt in ast {
    println!("{:#?}", stmt);
  }
}
