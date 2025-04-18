mod lexer;
mod linter;
mod parser;
mod ast; 
mod config;

use std::fs;
use std::env;

use lexer::{tokenize, TokenType};
use linter::lint_tokens;
use config::LinterConfig;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    // Create default config or load from file
    let config = if args.len() >= 3 && args[2] == "--config" && args.len() >= 4 {
        match LinterConfig::from_file(&args[3]) {
            Ok(cfg) => {
                println!("Using config from {}", args[3]);
                cfg
            },
            Err(e) => {
                println!("Error loading config file: {}.\nUsing default config.", e);
                LinterConfig::default()
            }
        }
    } else {
        LinterConfig::default()
    };
    
    if args.len() < 2 {
        println!("Usage: {} <file_path> [--config path/to/config.toml]", args[0]);
        std::process::exit(1);
        
    } 
    else {
        let file_path = &args[1];
        match fs::read_to_string(file_path) {
            Ok(content) => {
                println!("Linting file: {}", file_path);
                run_linter_on_code(&content, &config);
            },
            Err(e) => {
                println!("Error reading file {}: {}", file_path, e);
            }
        }
    }
}

fn run_linter_on_code(code: &str, config: &LinterConfig) {
    // Keywords from config for lexer
    let keywords = &config.keywords;
    
    // Pass the keywords to the lexer
    let tokens = tokenize(code, keywords);
    
    // Debug: Print tokens
    println!("Tokens found:");
    for token in &tokens {
        if token.token_type != TokenType::Whitespace {
            println!("{:?} {:?} at line {}, column {}", token.token_type, token.value, token.line, token.column);
        }
    }
    
    println!("\nRunning linter...");
    let diagnostics = lint_tokens(&tokens, config);
    
    if diagnostics.is_empty() {
        println!("No issues found!");
    } else {
        println!("\nIssues found:");
        for diagnostic in diagnostics {
            println!("{}", diagnostic);
        }
    }
}
