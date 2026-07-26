pub mod ast;
pub mod config;
pub mod errors;
pub mod lexer;
pub mod linter;
pub mod parser;
pub mod semantic;
pub mod utils;

use ast::Stmt;
use clap::Parser as ClapParser;
use config::Config;
use errors::{Diagnostic, Severity};
use lexer::Lexer;
use linter::registry::LintRuleRegistry;
use linter::rules::{
    CamelCaseRule, ConstantCaseRule, LineLengthRule, MaxFunctionLengthRule, MissingReturnRule,
    UnusedParametersRule, UnusedVariablesRule,
};
use parser::Parser;
use semantic::analyze;
use std::fs;
use std::process;
use std::time::Instant;
use utils::LineTracker;

#[derive(ClapParser)]
#[command(name = "blazelint")]
#[command(about = "A code linter for Ballerina programming language")]
#[command(version)]
pub struct Cli {
    /// The Ballerina file to lint
    pub file: String,

    /// Show timing information for each stage
    #[arg(short = 't', long)]
    pub timing: bool,

    /// Show detailed timing breakdown (lexer, parser, semantic, linter)
    #[arg(long)]
    pub detailed_timing: bool,

    /// Display tokens after lexing
    #[arg(long, alias = "st")]
    pub show_tokens: bool,

    /// Display Abstract Syntax Tree (AST) after parsing  
    #[arg(long, alias = "sa")]
    pub show_ast: bool,
}

pub fn run() {
    let args = Cli::parse();
    println!("Ballerina Linter (WIP)");

    let config = match config::load_config(None) {
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
        registry.register(Box::new(UnusedParametersRule));
        registry
    };

    let file_path = &args.file;
    let input_code = read_source(file_path);

    // Create LineTracker for efficient position mapping
    let line_tracker = LineTracker::new(&input_code);

    // Lexing stage with timing
    let (tokens, lex_duration) = {
        let start = Instant::now();
        let result = lex_input(&input_code);
        let duration = start.elapsed();
        match result {
            Ok(tokens) => (tokens, duration),
            Err(diagnostics) => {
                print_diagnostics(file_path, &input_code, &diagnostics, &line_tracker);
                process::exit(1);
            }
        }
    };

    if args.detailed_timing {
        println!("Lexing took: {:?} ({} tokens)", lex_duration, tokens.len());
    }

    if args.show_tokens {
        print_tokens(&tokens);
    }

    // Parsing stage with timing
    let ((ast, parse_diagnostics), parse_duration) = {
        let start = Instant::now();
        let result = parse_tokens(&tokens);
        let duration = start.elapsed();
        (result, duration)
    };

    if args.detailed_timing {
        println!("Parsing took: {:?}", parse_duration);
    }

    let mut all_diagnostics = Vec::new();
    all_diagnostics.extend(parse_diagnostics);

    // Initialize timing variables
    let (semantic_duration, lint_duration) = if !ast.is_empty() {
        // Semantic analysis stage with timing
        let semantic_duration = {
            let start = Instant::now();
            if let Err(semantic_diagnostics) = analyze(&ast, &line_tracker) {
                all_diagnostics.extend(semantic_diagnostics);
            }
            start.elapsed()
        };

        if args.detailed_timing {
            println!("Semantic analysis took: {:?}", semantic_duration);
        }

        if args.show_ast {
            print_ast(&ast);
        }

        // Linting stage with timing
        let (lint_diagnostics, lint_duration) = {
            let start = Instant::now();
            let diagnostics = run_linter(
                &lint_registry,
                &ast,
                file_path,
                &input_code,
                &config,
                &line_tracker,
            );
            let duration = start.elapsed();
            (diagnostics, duration)
        };

        if args.detailed_timing {
            println!("Linting took: {:?}", lint_duration);
        }

        all_diagnostics.extend(lint_diagnostics);
        (semantic_duration, lint_duration)
    } else {
        // If AST is empty, set durations to zero
        (std::time::Duration::ZERO, std::time::Duration::ZERO)
    };

    // Print diagnostics first
    if !all_diagnostics.is_empty() {
        print_diagnostics(file_path, &input_code, &all_diagnostics, &line_tracker);
    }

    // Show summary timing if requested (after diagnostics)
    if args.timing || args.detailed_timing {
        let total_duration = lex_duration + parse_duration + semantic_duration + lint_duration;
        println!("\n--- Timing Summary ---");
        println!("Total time: {:?} ({} tokens)", total_duration, tokens.len());
        if args.detailed_timing {
            println!(
                "  Lexer:    {:?} ({:.1}%)",
                lex_duration,
                (lex_duration.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0
            );
            println!(
                "  Parser:   {:?} ({:.1}%)",
                parse_duration,
                (parse_duration.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0
            );
            println!(
                "  Semantic: {:?} ({:.1}%)",
                semantic_duration,
                (semantic_duration.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0
            );
            println!(
                "  Linting:  {:?} ({:.1}%)",
                lint_duration,
                (lint_duration.as_nanos() as f64 / total_duration.as_nanos() as f64) * 100.0
            );
        }
        println!("----------------------");
    }

    // Exit with error code if there are errors
    if !all_diagnostics.is_empty()
        && all_diagnostics
            .iter()
            .any(|diag| diag.severity == Severity::Error)
    {
        process::exit(1);
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
    line_tracker: &LineTracker,
) -> Vec<Diagnostic> {
    registry.run_all(ast, file_path, source, config, line_tracker)
}

fn print_diagnostics(
    file_path: &str,
    _source: &str,
    diagnostics: &[Diagnostic],
    line_tracker: &LineTracker,
) {
    for diag in diagnostics {
        let severity_str = match diag.severity {
            Severity::Error => "Error",
            Severity::Warning => "Warning",
            Severity::Info => "Info",
        };
        println!("{}: {}", severity_str, diag.message);
        if let Some(pos) = diag.position {
            // Use pre-computed position when available (more efficient)
            println!("  --> {}:{}:{}", file_path, pos.line, pos.column);
        } else {
            // Fall back to LineTracker for better performance than old method
            let pos = line_tracker.byte_to_line_col(diag.span.start);
            println!("  --> {}:{}:{}", file_path, pos.line, pos.column);
        }
        for note in &diag.notes {
            println!("note: {}", note);
        }
    }
}
