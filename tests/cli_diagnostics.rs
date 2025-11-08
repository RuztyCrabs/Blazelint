use assert_cmd::Command;
use std::fs;
use std::process::Output;

// Comprehensive test file with all parsable syntax
const COMPREHENSIVE_TEST: &str = include_str!("test-bal-files/comprehensive_test.bal");

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

#[allow(dead_code)]
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

// ============================================================================
// INTEGRATION TEST
// ============================================================================

#[test]
fn comprehensive_test_passes() {
    let output = run_cli(COMPREHENSIVE_TEST);
    let out = stdout(&output);

    // Should complete all stages
    assert!(out.contains("Lexing complete!"), "Lexing should complete");
    assert!(out.contains("-- AST --"), "Should generate AST");

    // Should have no errors (check for the actual error format, not comments)
    assert!(!out.contains("Error:"), "Should have no errors");

    assert!(
        output.status.success(),
        "Comprehensive test should pass without errors"
    );
}

// ============================================================================
// LEXER TESTS
// ============================================================================

#[test]
fn lexer_tokenizes_imports() {
    let output = run_cli("import ballerina/io;");
    let out = stdout(&output);
    assert!(out.contains("Token: (0, Import, 6)"));
    assert!(out.contains("Token: (7, Identifier(\"ballerina\"), 16)"));
    assert!(out.contains("Token: (16, Slash, 17)"));
    assert!(out.contains("Token: (17, Identifier(\"io\"), 19)"));
}

#[test]
fn lexer_tokenizes_all_operators() {
    let code = "int x = 5 + 3 - 2 * 4 / 2 % 3;";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("Token: (10, Plus, 11)"));
    assert!(out.contains("Token: (14, Minus, 15)"));
    assert!(out.contains("Token: (18, Star, 19)"));
    assert!(out.contains("Token: (22, Slash, 23)"));
    assert!(out.contains("Token: (26, Percent, 27)"));
}

#[test]
fn lexer_tokenizes_bitwise_operators() {
    let code = "int x = 5 & 3 | 2 ^ 1; int y = ~x; int z = 4 << 2 >> 1;";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("Token: (10, Amp, 11)"));
    assert!(out.contains("Token: (14, Pipe, 15)"));
    assert!(out.contains("Token: (18, Caret, 19)"));
    assert!(out.contains("Token: (31, Tilde, 32)"));
    assert!(out.contains("Token: (45, LtLt, 47)"));
    assert!(out.contains("Token: (50, GtGt, 52)"));
}

#[test]
fn lexer_tokenizes_keywords() {
    let code = "function main() { if (true) { while (false) { } } }";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("Token: (0, Function, 8)"));
    assert!(out.contains("Token: (18, If, 20)"));
    assert!(out.contains("Token: (30, While, 35)"));
    assert!(out.contains("Token: (22, True, 26)"));
    assert!(out.contains("Token: (37, False, 42)"));
}

#[test]
fn lexer_reports_unterminated_string() {
    let output = run_cli("var a = \"unterminated;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Unterminated string literal"));
}

#[test]
fn lexer_reports_unterminated_block_comment() {
    let output = run_cli("var a = 1; /* unterminated block comment");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Unterminated block comment"));
}

#[test]
fn parser_reports_unexpected_bitwise_and() {
    let output = run_cli("int a = &;");
    assert!(!output.status.success());
    let out = stdout(&output);
    // Now that we support bitwise operators, single & is tokenized but creates parser error
    assert!(out.contains("Error:"));
}

#[test]
fn lexer_reports_malformed_exponent() {
    let output = run_cli("var a = 1e+;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Malformed exponent in number literal"));
}

#[test]
fn lexer_reports_unexpected_character() {
    let output = run_cli("var a = 1 @;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Unexpected character: '@'"));
}

// ============================================================================
// PARSER TESTS
// ============================================================================

#[test]
fn parser_handles_function_declarations() {
    let code = "function add(int a, int b) returns int { return a + b; }";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("Function {"));
    assert!(out.contains("name: \"add\""));
    assert!(!out.contains("parser error"));
}

#[test]
fn parser_handles_if_else_if_else() {
    let code = r#"
        function test(int x) { 
            if (x > 10) { 
                int y = 1; 
            } else if (x > 5) { 
                int y = 2; 
            } else { 
                int y = 3; 
            } 
        } 
    "#;
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("If {"));
    assert!(!out.contains("parser error"));
}

#[test]
fn parser_handles_while_loops() {
    let code = "function test() { int i = 0; while (i < 5) { i += 1; } }";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("While {"));
    assert!(!out.contains("parser error"));
}

#[test]
fn parser_handles_foreach_loops() {
    let code = "function test() { int[] nums = [1,2,3]; foreach int n in nums { int x = n; } }";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("Foreach {"));
    assert!(!out.contains("parser error"));
}

#[test]
fn parser_handles_ternary_operator() {
    let code = "function test(int x) returns int { return (x > 0) ? 1 : 0; }";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("Ternary {"));
    assert!(!out.contains("parser error"));
}

#[test]
fn parser_handles_arrays_and_maps() {
    let code = r#"
        function test() { 
            int[] arr = [1, 2, 3]; 
            map<string> m = {key: "value"};
        } 
    "#;
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(out.contains("ArrayLiteral {"));
    assert!(out.contains("MapLiteral {"));
    assert!(!out.contains("parser error"));
}

#[test]
fn parser_reports_missing_semicolon() {
    let output = run_cli("int a = 1\nint b = 2;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Expected ';' after variable declaration"));
}

#[test]
fn parser_recovers_from_multiple_errors() {
    // Test error recovery - should report parser errors but continue
    let code = "int a = 1\nint b = 2\nint c = 3;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    // Should catch semicolon error
    assert!(out.contains("Error:"));
    // Error recovery allows parser to continue
    assert!(out.contains("Lexing complete!"));
}

#[test]
fn parser_reports_invalid_assignment_target() {
    let output = run_cli("int a = 1; (a + 1) = 3;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Invalid assignment target"));
}

#[test]
fn parser_reports_missing_closing_paren() {
    let output = run_cli("int a = (1 + 2;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Expected ')' after expression"));
}

#[test]
fn parser_reports_unexpected_eof_in_block() {
    let output = run_cli("function foo() { int a = 1;");
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Expected '}' at end of block"));
    assert!(out.contains("note: expected: '}'"));
}

#[test]
fn parser_reports_const_with_type() {
    let code = "const int a = 1;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: const declarations cannot have a type annotation"));
}

// ============================================================================
// SEMANTIC TESTS
// ============================================================================

#[test]
fn semantic_reports_type_mismatch_in_assignment() {
    let code = "int a = 1; a = \"oops\";";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Type mismatch in assignment"));
}

#[test]
fn semantic_reports_final_reassignment() {
    let code = "final int a = 1; a = 2;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Cannot assign to final variable"));
}

#[test]
fn semantic_reports_missing_return_value() {
    let code = "function foo() returns int { return; }";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Missing return value"));
}

#[test]
fn semantic_reports_const_reassignment() {
    let code = "const a = 1; a = 2;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Cannot assign to constant"));
}

// ============================================================================
// LINTER TESTS
// ============================================================================

#[test]
fn linter_reports_line_length() {
    let code = "string long_line = \"this is a very long line that is longer than 120 characters just to test the line length rule in the linter, so that it will trigger the error and we can see the output of the linter\";";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Warning: Lines should not exceed 120 characters."));
    assert!(out.contains("Info: Variable \"long_line\" is not in camelCase."));
}

#[test]
fn linter_reports_camel_case() {
    let code = "int a_b = 1;";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Info: Variable \"a_b\" is not in camelCase."));
}

#[test]

fn linter_reports_constant_case() {
    let code = "const badConstant = 100;";

    let output = run_cli(code);

    assert!(output.status.success()); // Should succeed as it's an Info level diagnostic

    let out = stdout(&output);

    assert!(out.contains("Info: Constant variable \"badConstant\" is not in SCREAMING_SNAKE_CASE."));
}

#[test]
fn linter_accepts_valid_camel_case() {
    let code = "function test() { int myVariable = 42; string userName = \"test\"; io:println(myVariable); io:println(userName); }";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(!out.contains("linter error: Variable"));
}

#[test]
fn linter_accepts_valid_constant_case() {
    let code = "const MAX_SIZE = 100; const DEFAULT_NAME = \"test\";";
    let output = run_cli(code);
    let out = stdout(&output);
    assert!(!out.contains("linter error: Constant"));
    assert!(output.status.success(), "Valid constants should pass");
}

#[test]
fn linter_reports_max_function_length_with_empty_lines() {
    let mut code = "public function longFunction() {\n".to_string();
    for i in 0..49 {
        code.push_str(&format!("    int a{} = {};\n", i, i));
        if i % 2 == 0 {
            code.push('\n');
        }
    }
    code.push('}');

    let output = run_cli(&code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Warning: Function \"longFunction\" has 76 lines (exceeds maximum of 50)"));
}

// ============================================================================
// ERROR RECOVERY TESTS
// ============================================================================

#[test]
fn error_recovery_collects_multiple_parser_errors() {
    let code = "int a = 1\nint b = 2\nfunction test(x) { }";
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);

    // Should collect parser errors from multiple locations
    let error_count = out.matches("Error:").count();
    assert!(
        error_count >= 2,
        "Should report multiple parser errors, found: {}",
        error_count
    );
}

#[test]
fn error_recovery_runs_all_stages() {
    let code = r#"
        import ballerina/io;
        const badConstant = 10;
        int missing_semicolon = 5
        function test() { 
            io:println(nonExistent);
        }
    "#;
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);

    assert!(out.contains("Error:"), "Should have errors");
    assert!(out.contains(" -->"), "Should have positional information");
}

#[test]
fn error_recovery_still_parses_valid_code() {
    let code = r#"
        int badSyntax = 
        function goodFunc() returns int { return 42; } 
    "#;
    let output = run_cli(code);
    let out = stdout(&output);

    // Should have parser error for incomplete expression
    assert!(out.contains("Error:"));

    // Should still generate AST output (even if some nodes are skipped)
    assert!(
        out.contains("-- AST --"),
        "Should still generate AST despite errors"
    );
}

#[test]
fn linter_reports_max_function_length() {
    let code = include_str!("test-bal-files/long_function.bal");
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Warning: Function \"longFunction\" has 51 lines (exceeds maximum of 50)"));
    assert!(!out.contains("linter error: Function \"shortFunction\""));
}

#[test]
fn linter_reports_unused_variable() {
    let code = include_str!("test-bal-files/unused_variable.bal");
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Variable anotherUnused is never used"));
}

#[test]
fn linter_reports_missing_return() {
    let code = include_str!("test-bal-files/missing_return.bal");
    let output = run_cli(code);
    assert!(!output.status.success());
    let out = stdout(&output);
    assert!(out.contains("Error: Function 'getValue' might not return a value on all code paths."));
}

#[test]
fn test_all_linter_rules_triggered() {
    let code = include_str!("test-bal-files/all_rules_test.bal");
    let output = run_cli(code);
    let out = stdout(&output);

    // Assertions for each rule with their expected severity
    assert!(out.contains("Info: Constant variable \"badConstant\" is not in SCREAMING_SNAKE_CASE."));
    assert!(out.contains("Info: Variable \"variable_name\" is not in camelCase."));
    assert!(out.contains(
        "Error: Function 'missingReturnFunction' might not return a value on all code paths."
    ));
    assert!(out.contains("Error: Variable unused_var is never used"));
    assert!(out.contains(
        "Warning: Function \"longFunctionForLintTest\" has 53 lines (exceeds maximum of 50)"
    ));
    assert!(out.contains("Warning: Lines should not exceed 120 characters."));

    // The test should still fail because there are Error level diagnostics
    assert!(
        !output.status.success(),
        "Expected linter errors with Error severity"
    );
}