mod ast;
mod lexer;
mod parser;

use lexer::Lexer;
use parser::Parser;

/// Main function of the Blazelint.
///
/// This function initializes the lexer and parser, processes the input code,
/// and prints the generated tokens and Abstract Syntax Tree (AST).
fn main() {
  println!("Ballerina Linter (WIP)");

  // Example input code to be analyzed.
  let input_code = r#"
     // This is a test comment
        var myInt: int = 123;
        function calculate(a: float, b: float) returns float {
            /* Multi-
             line
             * comment */
            if (a > b || a != b) {
                return (a * b) / (a + b); // Complex expression
            } else {
                panic "Invalid operation!";
            }
        }
        var message: string = "Hello, Ballerina!";
        var isDone: boolean = true;
        var myFloat: float = 0.005e+2;
        another_ident_123 = 456;
  "#;

  println!("--- Input Code ---");
  println!("{}", input_code);
  println!("----------------------------\n");

  // Initialize the lexer with the input code and collect tokens.
  let lexer = Lexer::new(input_code);
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
  let ast = parser.parse();
  // Print the AST in a pretty-formatted way.
  for stmt in ast {
    println!("{:#?}", stmt);
  }
}
