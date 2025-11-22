use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

// Simple test file content for flag testing
const TEST_BALLERINA_CODE: &str = r#"
import ballerina/io;

function testFunction() {
    io:println("test");
}
"#;

// Helper function to create temp file and run command
fn run_blazelint_with_args(args: &[&str]) -> assert_cmd::assert::Assert {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    fs::write(temp_file.path(), TEST_BALLERINA_CODE).expect("Failed to write test content");

    Command::cargo_bin("blazelint")
        .expect("Failed to find blazelint binary")
        .args(args)
        .arg(temp_file.path())
        .assert()
}

// Helper for flags that don't require a file
fn run_blazelint_standalone(args: &[&str]) -> assert_cmd::assert::Assert {
    Command::cargo_bin("blazelint")
        .expect("Failed to find blazelint binary")
        .args(args)
        .assert()
}

#[test]
fn test_version_flag_long() {
    run_blazelint_standalone(&["--version"])
        .success()
        .stdout(predicates::str::contains("blazelint"))
        .stdout(predicates::str::contains("0.2.2"));
}

#[test]
fn test_version_flag_short() {
    run_blazelint_standalone(&["-V"])
        .success()
        .stdout(predicates::str::contains("blazelint"))
        .stdout(predicates::str::contains("0.2.2"));
}

#[test]
fn test_help_flag() {
    run_blazelint_standalone(&["--help"])
        .success()
        .stdout(predicates::str::contains(
            "A code linter for Ballerina programming language",
        ))
        .stdout(predicates::str::contains("Usage:"));
}

#[test]
fn test_help_flag_short() {
    run_blazelint_standalone(&["-h"])
        .success()
        .stdout(predicates::str::contains(
            "A code linter for Ballerina programming language",
        ))
        .stdout(predicates::str::contains("Usage:"));
}

#[test]
fn test_timing_flag_long() {
    run_blazelint_with_args(&["--timing"])
        .success()
        .stdout(predicates::str::contains("--- Timing Summary ---"))
        .stdout(predicates::str::contains("Total time:"))
        .stdout(predicates::str::contains("tokens)"));
}

#[test]
fn test_timing_flag_short() {
    run_blazelint_with_args(&["-t"])
        .success()
        .stdout(predicates::str::contains("--- Timing Summary ---"))
        .stdout(predicates::str::contains("Total time:"))
        .stdout(predicates::str::contains("tokens)"));
}

#[test]
fn test_detailed_timing_flag() {
    run_blazelint_with_args(&["--detailed-timing"])
        .success()
        .stdout(predicates::str::contains("Lexing took:"))
        .stdout(predicates::str::contains("tokens)"))
        .stdout(predicates::str::contains("Parsing took:"))
        .stdout(predicates::str::contains("Semantic analysis took:"))
        .stdout(predicates::str::contains("Linting took:"))
        .stdout(predicates::str::contains("--- Timing Summary ---"));
}

#[test]
fn test_show_tokens_flag_long() {
    run_blazelint_with_args(&["--show-tokens"])
        .success()
        .stdout(predicates::str::contains("---"))
        .stdout(predicates::str::contains("Token:"))
        .stdout(predicates::str::contains("Lexing complete!"));
}

#[test]
fn test_show_tokens_flag_alias() {
    run_blazelint_with_args(&["--st"])
        .success()
        .stdout(predicates::str::contains("---"))
        .stdout(predicates::str::contains("Token:"))
        .stdout(predicates::str::contains("Lexing complete!"));
}

#[test]
fn test_show_ast_flag_long() {
    run_blazelint_with_args(&["--show-ast"])
        .success()
        .stdout(predicates::str::contains("-- AST --"))
        .stdout(predicates::str::contains("Import"))
        .stdout(predicates::str::contains("Function"));
}

#[test]
fn test_show_ast_flag_alias() {
    run_blazelint_with_args(&["--sa"])
        .success()
        .stdout(predicates::str::contains("-- AST --"))
        .stdout(predicates::str::contains("Import"))
        .stdout(predicates::str::contains("Function"));
}

#[test]
fn test_combined_flags_timing_and_tokens() {
    run_blazelint_with_args(&["--timing", "--show-tokens"])
        .success()
        .stdout(predicates::str::contains("Token:"))
        .stdout(predicates::str::contains("--- Timing Summary ---"))
        .stdout(predicates::str::contains("Total time:"))
        .stdout(predicates::str::contains("tokens)"));
}

#[test]
fn test_combined_flags_detailed_timing_and_ast() {
    run_blazelint_with_args(&["--detailed-timing", "--show-ast"])
        .success()
        .stdout(predicates::str::contains("Lexing took:"))
        .stdout(predicates::str::contains("-- AST --"))
        .stdout(predicates::str::contains("Function"));
}

#[test]
fn test_combined_short_flags() {
    run_blazelint_with_args(&["-t", "--st", "--sa"])
        .success()
        .stdout(predicates::str::contains("Token:"))
        .stdout(predicates::str::contains("-- AST --"))
        .stdout(predicates::str::contains("--- Timing Summary ---"))
        .stdout(predicates::str::contains("tokens)"));
}

#[test]
fn test_all_flags_together() {
    run_blazelint_with_args(&["--detailed-timing", "--show-tokens", "--show-ast"])
        .success()
        .stdout(predicates::str::contains("Lexing took:"))
        .stdout(predicates::str::contains("tokens)"))
        .stdout(predicates::str::contains("Token:"))
        .stdout(predicates::str::contains("-- AST --"))
        .stdout(predicates::str::contains("--- Timing Summary ---"));
}

#[test]
fn test_clean_output_no_flags() {
    run_blazelint_with_args(&[])
        .success()
        .stdout(predicates::str::contains("Ballerina Linter (WIP)"))
        // Should NOT contain debug output when no flags are provided
        .stdout(predicates::str::contains("Token:").not())
        .stdout(predicates::str::contains("-- AST --").not())
        .stdout(predicates::str::contains("--- Timing Summary ---").not());
}

#[test]
fn test_invalid_flag() {
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    fs::write(temp_file.path(), TEST_BALLERINA_CODE).expect("Failed to write test content");

    Command::cargo_bin("blazelint")
        .expect("Failed to find blazelint binary")
        .arg("--invalid-flag")
        .arg(temp_file.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("unexpected argument"));
}

#[test]
fn test_missing_file_argument() {
    Command::cargo_bin("blazelint")
        .expect("Failed to find blazelint binary")
        .arg("--timing")
        .assert()
        .failure()
        .stderr(predicates::str::contains(
            "required arguments were not provided",
        ))
        .stderr(predicates::str::contains("<FILE>"));
}
