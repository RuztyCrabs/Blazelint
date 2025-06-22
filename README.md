<div align="center">

# BlazeLint
<p> -- WIP Linter for Ballerina --  </p>

</div>

## Setup

1. Clone the repo:

```bash
git clone https://github.com/RuztyCrabs/Blazelint.git
```

2. `cd` into the directory:

```bash
cd Blazeint
```

3. Run in debug mode:

```bash
cargo run blazelint
```

## Todo

1. [ ] **Lexer** – Convert source code into a stream of tokens  
   - [x] Single-character tokens (`+`, `-`, `;`, etc.)
   - [x] Double-character tokens (`==`, `!=`, etc.)
   - [x] Number literals
   - [ ] Identifiers and keywords
   - [ ] Strings
   - [ ] Comments
2. [ ] **Parser** – Convert tokens into an Abstract Syntax Tree (AST)  
   - [ ] Expressions
   - [ ] Statements (if, while, etc.)
   - [ ] Declarations (variables, functions)
3. [ ] **AST Representation** – Define node types for expressions and statements  
4. [ ] **Semantic Analyzer** – Validate and annotate the AST  
   - [ ] Scope and symbol tracking
   - [ ] Type checks (if needed)
5. [ ] **Linter Passes** – Implement actual lint rules  
   - [ ] Unused variable detection
   - [ ] Variable shadowing
   - [ ] Unreachable code
   - [ ] Naming conventions
6. [ ] **Error Reporting** – Show clear messages with line/column info  
7. [ ] **Configuration Support** – Allow enabling/disabling lints via config file (`.blazelint.toml`)  
8. [ ] **Testing Framework** – Unit + integration tests for each component  
9. [ ] **Command-Line Interface (CLI)** – Input/output via files or stdin  
10. [ ] **Performance Optimizations** – Handle large files efficiently  
11. [ ] **Documentation & Examples** – Include usage guide and rule reference
