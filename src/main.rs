mod ast;
mod errors;
mod lexer;
mod linter;
mod parser;
mod semantic;
mod utils;

use ast::Stmt;
use errors::{Diagnostic, Severity}; // Import Severity
use lexer::Lexer;
use linter::registry::LintRuleRegistry;
use linter::rules::{
    CamelCaseRule, ConstantCaseRule, LineLengthRule, MaxFunctionLengthRule, MissingReturnRule,
    UnusedVariablesRule,
};
use once_cell::sync::Lazy;
use parser::Parser;
use semantic::analyze;
use std::env;
use std::fs;
use std::process;

static LINT_REGISTRY: Lazy<LintRuleRegistry> = Lazy::new(|| {
    let mut registry = LintRuleRegistry::new();
    registry.register(Box::new(CamelCaseRule));
    registry.register(Box::new(ConstantCaseRule));
    registry.register(Box::new(LineLengthRule));
    registry.register(Box::new(MaxFunctionLengthRule::new()));
    registry.register(Box::new(MissingReturnRule::new()));
    registry.register(Box::new(UnusedVariablesRule));
    registry
});

/// Main entrypoint of the Blazelint linter.
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
    let input_code = read_source(file_path);
    let tokens = match lex_input(&input_code) {
        Ok(tokens) => tokens,
        Err(diagnostics) => {
            print_diagnostics(&input_code, &diagnostics);
            // If lexing fails, it's always a critical error
            process::exit(1);
        }
    };
    print_tokens(&tokens);
    let (ast, parse_diagnostics) = parse_tokens(&tokens);
    // Collect all diagnostics
    let mut all_diagnostics = Vec::new();
    // Add parser errors
    all_diagnostics.extend(parse_diagnostics);
    // Run semantic analysis if we have any AST
    if !ast.is_empty() {
        if let Err(semantic_diagnostics) = analyze(&ast) {
            all_diagnostics.extend(semantic_diagnostics);
        }
        print_ast(&ast);
        // Run linter rules even if there are errors (to catch style issues)
        all_diagnostics.extend(run_linter(&LINT_REGISTRY, &ast, file_path, &input_code));
    }
    // Display all collected diagnostics
    if !all_diagnostics.is_empty() {
        print_diagnostics(&input_code, &all_diagnostics);

        // Exit with error code if any diagnostic has Severity::Error
        if all_diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error)
        {
            process::exit(1);
        }
    }
}

//---------------------------------- Helpers --------------------------------------------------------------------

fn read_source(path: &str) -> String {
    match fs::read_to_string(path) {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error reading file {}: {}", path, err);
            process::exit(1);
        }
    }
}

fn lex_input(input: &str) -> Result<Vec<(usize, lexer::Token, usize)>, Vec<Diagnostic>> {
    let lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    for result in lexer {
        match result {
            Ok(token) => tokens.push(token),
            Err(diagnostic) => diagnostics.push(diagnostic.into()),
        }
    }
    if diagnostics.is_empty() {
        Ok(tokens)
    } else {
        Err(diagnostics)
    }
}

fn parse_tokens(tokens: &[(usize, lexer::Token, usize)]) -> (Vec<Stmt>, Vec<Diagnostic>) {
    let parser = Parser::new(tokens.to_vec());
    parser.parse()
}

fn print_tokens(tokens: &[(usize, lexer::Token, usize)]) {
    println!("--- Tokens ---");
    for token in tokens {
        println!("Token: {:?}", token);
    }
    println!("----------------------------\n");
    println!("Lexing complete!");
}

fn print_ast(ast: &[Stmt]) {
    println!("-- AST --");
    for stmt in ast {
        println!("{:#?}", stmt);
    }
}

/// This function iterates through each statement in the AST and applies a predefined
/// set of linting rules. If any rule violations are found, they are collected and
/// printed to the console using the provided source code and line information for
/// context.
///
/// # Args
///
/// * `ast` - A slice of `Stmt` representing the AST to be linted.
/// * `file_path` - The path to the file being linted.
/// * `source` - The source code string, used for displaying diagnostic messages.
fn run_linter(
    registry: &LintRuleRegistry,
    ast: &[Stmt],
    file_path: &str,
    source: &str,
) -> Vec<Diagnostic> {
    registry.run_all(ast, file_path, source)
}

fn print_diagnostics(source: &str, diagnostics: &[Diagnostic]) {
    for diag in diagnostics {
        let severity_str = match diag.severity {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
        };
        println!("{}: {}", severity_str, diag.message);
        if let Some(pos) = diag.position {
            println!("  --> {}:{}:{}", pos.line, pos.column, diag.message);
        } else {
            let pos = crate::utils::get_line_and_column(diag.span.start, source);
            println!("  --> {}:{}:{}", pos.line, pos.column, diag.message);
        }
        for note in &diag.notes {
            println!("note: {}", note);
        }
    }
}
