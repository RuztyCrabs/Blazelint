pub mod ast;
pub mod config;
pub mod errors;
pub mod lexer;
pub mod linter;
pub mod parser;
pub mod semantic;
pub mod utils;

use ast::Stmt;
use config::Config;
use errors::{Diagnostic, Severity};
use lexer::Lexer;
use linter::registry::LintRuleRegistry;
use linter::rules::{
    CamelCaseRule, ConstantCaseRule, LineLengthRule, MaxFunctionLengthRule, MissingReturnRule,
    UnusedVariablesRule,
};
use parser::Parser;
use semantic::analyze;
use std::env;
use std::fs;
use std::process;

pub fn run() {
    println!("Ballerina Linter (WIP)");

    let config = match config::load_config() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("Error loading configuration: {}", err);
            process::exit(1);
        }
    };

    let lint_registry = {
        let mut registry = LintRuleRegistry::new();
        registry.register(Box::new(CamelCaseRule));
        registry.register(Box::new(ConstantCaseRule));
        registry.register(Box::new(LineLengthRule));
        registry.register(Box::new(MaxFunctionLengthRule::new()));
        registry.register(Box::new(MissingReturnRule::new()));
        registry.register(Box::new(UnusedVariablesRule));
        registry
    };

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
            print_diagnostics(file_path, &input_code, &diagnostics);
            process::exit(1);
        }
    };
    print_tokens(&tokens);
    let (ast, parse_diagnostics) = parse_tokens(&tokens);
    let mut all_diagnostics = Vec::new();
    all_diagnostics.extend(parse_diagnostics);
    if !ast.is_empty() {
        if let Err(semantic_diagnostics) = analyze(&ast) {
            all_diagnostics.extend(semantic_diagnostics);
        }
        print_ast(&ast);
        all_diagnostics.extend(run_linter(
            &lint_registry,
            &ast,
            file_path,
            &input_code,
            &config,
        ));
    }
    if !all_diagnostics.is_empty() {
        print_diagnostics(file_path, &input_code, &all_diagnostics);

        if all_diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error)
        {
            process::exit(1);
        }
    }
}

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
    println!("---");
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

fn run_linter(
    registry: &LintRuleRegistry,
    ast: &[Stmt],
    file_path: &str,
    source: &str,
    config: &Config,
) -> Vec<Diagnostic> {
    registry.run_all(ast, file_path, source, config)
}

fn print_diagnostics(file_path: &str, source: &str, diagnostics: &[Diagnostic]) {
    for diag in diagnostics {
        let severity_str = match diag.severity {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
        };
        println!("{}: {}", severity_str, diag.message);
        if let Some(pos) = diag.position {
            println!("  --> {}:{}:{}", file_path, pos.line, pos.column);
        } else {
            let pos = crate::utils::get_line_and_column(diag.span.start, source);
            println!("  --> {}:{}:{}", file_path, pos.line, pos.column);
        }
        for note in &diag.notes {
            println!("note: {}", note);
        }
    }
}
