# Software Requirements Specification (SRS): BlazeLint

**Date** &nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;: 22.04.2025 <br>
**Verson** &nbsp;&nbsp;&nbsp;: 1.0.0 <br>
**Authors** &nbsp;: M. C. R. Mallawaarachchi, R. K. N. R. Ranasinghe, G. K. S. Pathum <br>

# 1. Introduction

## 1.1 Purpose

The purpose of this project is to develop a modular, extensible and highly pluggable code linter for Ballerina programming language, written in Rust. It will parse code into tokens and apply custom linting rules to identify common errors, bad practices, or violations of style guidelines.

## 1.2 Scope
This linter is designed to:
- Tokenize source code via a lexer
- Apply multiple linting rules (e.g., unknown tokens, bad declarations)
-  Report diagnostics such as warnings and errors
- Be extendable with new rules via a trait-based plugin architecture

## 1.3 Terminology

| Term       | Definition                                                   |
|------------|--------------------------------------------------------------|
| AST        | Abstract Syntax Tree                                         |
| Token      | A unit of code (keyword, symbol, identifier, etc.)           |
| Diagnostic | A warning or error reported by a rule                        |
| Rule       | A module that checks specific patterns or errors in the code |

# 2. Overall Description

## 2.1 Product Perspective
This tool is a standalone command-line application or library. It can be integrated with IDEs or used in CI pipelines.

## 2.2 Product Functions
1. **Lexical analysis** : Tokenizes source code
2. **Rule engine** : Applies a list of rules to token slices
3. **Diagnostics reporter** : Outputs structured issues
4. **Configuration** : Enables or disables specific rules

2.3 User Classes and Characteristics
Developers who want static code analysis

Educators teaching language semantics

Contributors adding new rules via trait implementations

2.4 Operating Environment
Rust compiler (edition 2021+)

Cross-platform (Linux, Windows, macOS)

CLI or integration via library
