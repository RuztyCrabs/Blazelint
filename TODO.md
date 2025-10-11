## Project Roadmap 

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
- [x] Structured lexer/parser error propagation with span-aware diagnostics
- [x] Semantic analysis pass validating scope, type compatibility, finals, and returns
- [x] Diagnostic reporting pipeline that surfaces lex/parse/semantic issues with span highlights

### Remainings for the MVP: 

- [ ] **Linter Rules:** Define and implement specific linting rules (e.g., naming conventions, code style, best practices).
- [ ] **Reporting/Output:** Create a mechanism to report linting issues to the user (e.g., nicely formatted console output).
- [ ] **Configuration:** Allow users to configure linting rules using .blazerc file(e.g., enable/disable rules, set severity).
- [ ] **CLI Arguments:** Handle command-line arguments for specifying files/directories to lint, configuration files, etc.