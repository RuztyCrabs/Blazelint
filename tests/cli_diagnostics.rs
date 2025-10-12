use assert_cmd::Command;
use std::fs;
use std::process::Output;

const SAMPLE_SUCCESS: &str = include_str!("test.bal");

fn run_cli(source: &str) -> Output {
    let file = tempfile::NamedTempFile::new().expect("create temp file");
    fs::write(file.path(), source).expect("write temp source");
    Command::cargo_bin("blazelint")
        .expect("binary")
        .arg(file.path())
        .output()
        .expect("run blazelint")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn passes_on_valid_program() {
    let output = run_cli(SAMPLE_SUCCESS);
    assert!(
        output.status.success(),
        "expected success, got: {:?}",
        output.status
    );
    let out = stdout(&output);
    assert!(out.contains("Lexing complete!"));
    assert!(out.contains("-- AST --"));
    assert!(out.contains("Function {"));
}

#[test]
fn lexer_reports_unterminated_string() {
    let output = run_cli("var a = \"unterminated;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("lexer error: Unterminated string literal"));
    assert!(out.contains("^"));
}

#[test]
fn lexer_reports_unterminated_block_comment() {
    let output = run_cli("var a = 1; /* unterminated block comment");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("lexer error: Unterminated block comment"));
}

#[test]
fn lexer_reports_single_ampersand() {
    let output = run_cli("var a = &;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("lexer error: Unexpected single '&' character"));
}

#[test]
fn lexer_reports_malformed_exponent() {
    let output = run_cli("var a = 1e+;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("lexer error: Malformed exponent in number literal"));
}

#[test]
fn lexer_reports_unexpected_character() {
    let output = run_cli("var a = 1 @;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("lexer error: Unexpected character: '@'"));
}

#[test]
fn parser_reports_missing_semicolon() {
    let output = run_cli("var a = 1\nvar b = 2;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("parser error: Expected ';' after variable declaration"));
    assert!(out.contains("note: expected: ';'"));
}

#[test]
fn parser_reports_invalid_assignment_target() {
    let output = run_cli("var a = 1; (a + 1) = 3;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("parser error: Invalid assignment target"));
}

#[test]
fn parser_reports_missing_closing_paren() {
    let output = run_cli("var a = (1 + 2;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("parser error: Expected ')' after expression"));
}

#[test]
fn parser_reports_unexpected_eof_in_block() {
    let output = run_cli("function foo() { var a = 1;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("parser error: Expected '}' at end of block"));
    assert!(out.contains("note: expected: '}'"));
}

#[test]
fn semantic_reports_type_mismatch_in_assignment() {
    let code = "int a = 1; a = \"oops\";";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("semantic error: Type mismatch in assignment"));
}

#[test]
fn semantic_reports_final_reassignment() {
    let code = "final int a = 1; a = 2;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("semantic error: Cannot assign to final variable"));
}

#[test]
fn semantic_reports_missing_return_value() {
    let code = "function foo() returns int { return; }";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("semantic error: Missing return value"));
}

#[test]
fn semantic_reports_const_reassignment() {
    let code = "const a = 1; a = 2;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("semantic error: Cannot assign to constant"));
}

#[test]
fn parser_reports_const_with_type() {
    let code = "const int a = 1;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("parser error: const declarations cannot have a type annotation"));
}
