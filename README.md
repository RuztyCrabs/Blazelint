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

3. Run :

```bash
cargo run test.bal
```

## Documentation

*   [Explanation of Blazelint's Architecture and Design](explain.md)
*   [Backus-Naur Form (BNF) Grammar for Ballerina Subset](BNF.md)

# TODO

### Completed:

- [x] Lexer: Fully implemented and capable of tokenizing Ballerina syntax.
- [x] Parser: Implemented with support for:
  - [x] Variable declarations
  - [x] Function declarations
  - [x] If statements
  - [x] Return statements
  - [x] Panic statements
  - [x] Basic expressions (equality, comparison, term, factor, unary, primary)
  - [x] Logical OR (`||`) and AND (`&&`) operators
  - [x] Assignment expressions
  - [x] Type parsing

### Remaining for MVP Linter:

- [ ] **Error Handling and Reporting:** Implement graceful error recovery and provide informative error messages instead of panicking.
- [ ] **Semantic Analysis:** Develop checks for type consistency, variable scope, unused variables, and other semantic rules.
- [ ] **Linter Rules:** Define and implement specific linting rules (e.g., naming conventions, code style, best practices).
- [ ] **Reporting/Output:** Create a mechanism to report linting issues to the user (e.g., console output, SARIF format).
- [ ] **Configuration:** Allow users to configure linting rules (e.g., enable/disable rules, set severity).
- [ ] **CLI Arguments:** Handle command-line arguments for specifying files/directories to lint, configuration files, etc.
