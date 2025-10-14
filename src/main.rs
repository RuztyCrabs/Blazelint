mod ast;
mod errors;
mod lexer;
mod linter;
mod parser;
mod semantic;

use ast::Stmt;
use errors::{Diagnostic, DiagnosticKind};
use lexer::Lexer;
use linter::{
    rules::camel_case::CamelCase,
    rules::constant_case::ConstantCase,
    rules::line_length::LineLength, // Import the new rule
    Rule,
};
use parser::Parser;
use semantic::analyze;
use std::env;
use std::fs;
use std::process;

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

    print_input(&input_code);

    let line_starts = compute_line_starts(&input_code);

    let tokens = match lex_input(&input_code) {
        Ok(tokens) => tokens,
        Err(diagnostics) => {
            exit_with_diagnostics(&input_code, &line_starts, diagnostics);
            process::exit(1);
        }
    };

    print_tokens(&tokens);

    match parse_tokens(&tokens) {
        Ok(ast) => {
            if let Err(diagnostics) = analyze(&ast) {
                exit_with_diagnostics(&input_code, &line_starts, diagnostics);
                process::exit(1);
            }
            print_ast(&ast);
            if let Err(diagnostics) = run_linter(&ast, &input_code, &line_starts) {
                exit_with_diagnostics(&input_code, &line_starts, diagnostics);
                process::exit(1);
            }
        }
        Err(diagnostic) => {
            exit_with_diagnostics(&input_code, &line_starts, vec![diagnostic]);
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

fn print_input(source: &str) {
    println!("--- Input Code ---");
    println!("{}", source);
    println!("----------------------------\n");
}

fn lex_input(input: &str) -> Result<Vec<(usize, lexer::Token, usize)>, Vec<Diagnostic>> {
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();

    for result in Lexer::new(input) {
        match result {
            Ok(token) => tokens.push(token),
            Err(err) => diagnostics.push(err.into()),
        }
    }

    if diagnostics.is_empty() {
        Ok(tokens)
    } else {
        Err(diagnostics)
    }
}

fn parse_tokens(tokens: &[(usize, lexer::Token, usize)]) -> Result<Vec<Stmt>, Diagnostic> {
    let mut parser = Parser::new(tokens.to_vec());
    parser.parse().map_err(|e| e.into())
}

fn print_tokens(tokens: &[(usize, lexer::Token, usize)]) {
    println!("--- Tokens ---");
    for (start, token, end) in tokens {
        println!("Token: {:?} ({}..{})", token, start, end);
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

///
/// This function iterates through each statement in the AST and applies a predefined
/// set of linting rules. If any rule violations are found, they are collected and
/// printed to the console using the provided source code and line information for
/// context.
///
/// # Args
///
/// * `ast` - A slice of `Stmt` representing the AST to be linted.
/// * `source` - The source code string, used for displaying diagnostic messages.
/// * `line_starts` - A slice of byte offsets, where each offset is the start of a new line.
///   This is used to convert a diagnostic's position into a line and column number.
fn run_linter(ast: &[Stmt], source: &str, _line_starts: &[usize]) -> Result<(), Vec<Diagnostic>> {
    // if you add a new rule then add that same as the CamelCase
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(CamelCase),
        Box::new(ConstantCase),
        Box::new(LineLength),
    ];

    let mut diagnostics = Vec::new();

    for stmt in ast {
        for rule in &rules {
            diagnostics.extend(rule.validate(stmt, source));
        }
    }

    if !diagnostics.is_empty() {
        Err(diagnostics)
    } else {
        Ok(())
    }
}

fn exit_with_diagnostics(source: &str, line_starts: &[usize], diagnostics: Vec<Diagnostic>) {
    print_diagnostics(source, line_starts, &diagnostics);
}

/// Computes the byte indices where each line in `source` begins, including a
/// sentinel entry for the end of the file.
fn compute_line_starts(source: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (idx, ch) in source.char_indices() {
        if ch == '\n' {
            starts.push(idx + ch.len_utf8());
        }
    }
    starts.push(source.len());
    starts
}

/// Converts a byte index into a 1-based (line, column) pair using the provided
/// line-start table.
fn byte_to_line_col(line_starts: &[usize], index: usize) -> (usize, usize) {
    if line_starts.len() <= 1 {
        return (1, index + 1);
    }

    for (line_idx, window) in line_starts.windows(2).enumerate() {
        if index < window[1] {
            let col = index.saturating_sub(window[0]) + 1;
            return (line_idx + 1, col);
        }
    }

    let last_idx = line_starts.len().saturating_sub(2);
    let line_start = line_starts.get(last_idx).copied().unwrap_or(0);
    (last_idx + 1, index.saturating_sub(line_start) + 1)
}

/// Retrieves the text of a specific 1-based line number, trimming trailing
/// newline characters.
fn line_text(source: &str, line_starts: &[usize], line: usize) -> String {
    if line == 0 || line >= line_starts.len() {
        return String::new();
    }

    let start = line_starts[line - 1].min(source.len());
    let end = line_starts[line].min(source.len());
    let mut text = source[start..end].to_string();
    while text.ends_with('\n') || text.ends_with('\r') {
        text.pop();
    }
    text
}

/// Builds a caret marker string that highlights the relevant portion of the
/// line corresponding to `span`.
fn build_highlight_line(
    source: &str,
    span_start: usize,
    span_end: usize,
    line_start: usize,
    line_end: usize,
) -> String {
    let highlight_start = span_start.clamp(line_start, line_end);
    let highlight_end = span_end.clamp(highlight_start, line_end);

    let prefix_slice = &source[line_start..highlight_start];
    let highlight_slice = &source[highlight_start..highlight_end];

    let prefix_chars = prefix_slice.chars().count();
    let highlight_chars = highlight_slice.chars().count().max(1);

    let spaces = " ".repeat(prefix_chars);
    let carets = "^".repeat(highlight_chars);

    format!("{}{}", spaces, carets)
}

/// Prints diagnostics with line/column information and relevant source snippets.
fn print_diagnostics(source: &str, line_starts: &[usize], diagnostics: &[Diagnostic]) {
    for diag in diagnostics {
        let source_len = source.len();
        let span_start = diag.span.start.min(source_len);
        let span_end = diag.span.end.min(source_len);

        let (start_line, start_col) = byte_to_line_col(line_starts, span_start);
        let (end_line, end_col) = byte_to_line_col(line_starts, span_end);

        let kind = match diag.kind {
            DiagnosticKind::Lex => "lexer",
            DiagnosticKind::Parse => "parser",
            DiagnosticKind::Semantic => "semantic",
            DiagnosticKind::Linter => "linter",
        };

        println!("{kind} error: {}", diag.message);
        println!(" --> {}:{}-{}:{}", start_line, start_col, end_line, end_col);

        let line_label = format!("{:>4}", start_line);
        let line_start_idx = line_starts[start_line - 1].min(source_len);
        let line_end_idx = line_starts[start_line].min(source_len);
        let text = line_text(source, line_starts, start_line);
        println!("{line_label} | {text}");

        let highlight =
            build_highlight_line(source, span_start, span_end, line_start_idx, line_end_idx);
        println!("     | {highlight}");

        if start_line != end_line {
            println!("     | (continues to line {} column {})", end_line, end_col);
        }

        for note in &diag.notes {
            println!(" note: {}", note);
        }

        println!();
    }
}
