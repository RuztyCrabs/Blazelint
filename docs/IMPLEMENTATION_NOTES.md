# Blazelint Implementation Notes

## Overview

Blazelint is a linter for the Ballerina programming language, implemented in Rust. This document provides implementation details, architecture notes, and guidance for future development.

**Last Updated:** October 14, 2025  
**Branch:** rukshan-pr-1

---

## Recent Updates: BNF Grammar Expansion

The codebase was recently updated to support an expanded Ballerina grammar as defined in `BNF.md`. The implementation now supports:

### ✅ Completed Features

#### 1. Lexer Enhancements (`src/lexer.rs`)

**New Keywords:**
- `import`, `public` - Module system
- `decimal`, `byte`, `anydata`, `map` - Additional types
- `break`, `continue` - Loop control
- `is` - Type checking operator

**New Operators:**
- Arithmetic: `%` (modulo)
- Equality: `===`, `!==` (strict equality)
- Bitwise: `&`, `|`, `^`, `~`
- Shift: `<<`, `>>`, `>>>`
- Compound assignment: `+=`, `-=`
- Ternary: `?`, `?:` (Elvis operator)
- Range: `...`

**New Delimiters:**
- `[`, `]` - Array indexing and literals
- `.` - Member access and method calls
- `` ` `` - String templates (backtick)

**String Templates:**
```ballerina
string name = "World";
string greeting = `Hello, ${name}!`;
```
- Lexer captures template content including `${}` interpolations
- Currently treated as string literals in semantic analysis

#### 2. AST Expansions (`src/ast.rs`)

**Type System (`TypeDescriptor` enum):**
```rust
pub enum TypeDescriptor {
    Basic(String),                          // int, string, boolean, etc.
    Array {
        element_type: Box<TypeDescriptor>,
        dimension: Option<ArrayDimension>,  // [3], [], [*]
    },
    Map {
        value_type: Box<TypeDescriptor>,    // map<string>
    },
    Optional(Box<TypeDescriptor>),          // int?
    Union(Vec<TypeDescriptor>),             // int|string
}
```

**Array Dimensions:**
- `Fixed(usize)` - `int[3]` (fixed size)
- `Open` - `int[]` (dynamic size)
- `Inferred` - `int[*]` (inferred from initializer)

**New Expression Types:**
- `MemberAccess` - `arr[0]`, `obj.field`
- `MethodCall` - `arr.push(x)`, `str.length()`
- `ArrayLiteral` - `[1, 2, 3]`
- `MapLiteral` - `{key: "value"}`
- `Ternary` - `condition ? true_val : false_val`
- `Elvis` - `nullable_val ?: default_val`
- `Range` - `1...10`
- `Cast` - `<int>value`

**New Statement Types:**
- `Import { package_path: Vec<String>, span }` - `import ballerina/io;`
- `While { condition, body, span }` - While loops
- `Foreach { type_annotation, variable, iterable, body, span }` - Foreach loops
- `Break { span }`, `Continue { span }` - Loop control

**Extended Operators:**
```rust
pub enum BinaryOp {
    // Arithmetic
    Plus, Minus, Star, Slash, Percent,
    
    // Comparison
    EqualEqual, NotEqual, EqualEqualEqual, NotEqualEqual,
    Greater, GreaterEqual, Less, LessEqual, Is,
    
    // Logical
    And, Or,
    
    // Bitwise
    BitwiseAnd, BitwiseOr, BitwiseXor,
    
    // Shift
    LeftShift, RightShift, UnsignedRightShift,
    
    // Assignment
    PlusAssign, MinusAssign,
}

pub enum UnaryOp {
    Bang, Minus, Plus, BitwiseNot,
}
```

#### 3. Parser Implementation (`src/parser.rs`)

**Key Functions:**

1. **`parse_type_descriptor()`** - Parses complex type expressions:
   ```ballerina
   int[3]              // Array with fixed size
   int[]               // Open array
   int[*]              // Inferred size array
   map<string>         // Map type
   int?                // Optional type
   int|string          // Union type
   ```

2. **`import_declaration()`** - Handles imports:
   ```ballerina
   import ballerina/io;
   import org/package/module;
   ```

3. **`call()` (postfix expressions)** - Handles:
   - Method calls: `obj.method(args)`
   - Array/map access: `arr[index]`, `map[key]`
   - Qualified calls: `module:function(args)`
   - Chained operations: `arr.filter(fn).map(fn2)`

4. **`starts_var_decl()`** - Smart lookahead for variable declarations:
   - Handles `int[3] numbers = [1, 2, 3];`
   - Skips type suffixes (`[]`, `?`, `|`) to find identifier
   - Supports map types `map<T> x`

5. **`primary()`** - Extended to parse:
   - Array literals: `[expr, expr, ...]`
   - Map literals: `{key: value, ...}`
   - String templates: `` `text ${expr} text` ``

**Qualified Function Calls:**
```ballerina
io:println("Hello");  // Parsed as Call with qualified name "io:println"
```

#### 4. Semantic Analysis (`src/semantic.rs`)

**Type Conversion:**
- Now uses `TypeDescriptor` instead of `String` for type annotations
- `type_from_annotation()` converts `TypeDescriptor` to internal `Type` enum

**Statement Handlers:**
- `Import` - No-op (imports are declarations only)
- `While` - Validates boolean condition
- `Foreach` - Creates loop variable in new scope
- `Break`/`Continue` - No validation (TODO: check they're in loops)

**Expression Type Checking:**
- New operators: `%`, `===`, `!==`, `is`, bitwise, shift
- Ternary: Validates boolean condition, returns compatible type
- Elvis: Returns type of first operand
- Member access, method calls, arrays, maps: Return `Unknown` (TODO)
- Cast: Returns the target type

**Current Limitations:**
```rust
// These return Type::Unknown and need full implementation:
Expr::MemberAccess { .. } => Type::Unknown("member_access".to_string()),
Expr::MethodCall { .. } => Type::Unknown("method_call".to_string()),
Expr::ArrayLiteral { .. } => Type::Unknown("array".to_string()),
Expr::MapLiteral { .. } => Type::Unknown("map".to_string()),
Expr::Range { .. } => Type::Unknown("range".to_string()),
```

---

## Architecture Overview

### Pipeline Flow

```
Source File (.bal)
    ↓
┌─────────────┐
│   Lexer     │  → Stream of (start, Token, end) tuples
└─────────────┘
    ↓
┌─────────────┐
│   Parser    │  → Abstract Syntax Tree (Vec<Stmt>)
└─────────────┘
    ↓
┌─────────────┐
│  Semantic   │  → Type checking, scope analysis
│  Analyzer   │
└─────────────┘
    ↓
┌─────────────┐
│   Linter    │  → Style/convention rules
│   Rules     │
└─────────────┘
    ↓
Diagnostics or Success
```

### Module Structure

```
src/
├── main.rs           - Entry point, pipeline orchestration
├── lexer.rs          - Tokenization
├── parser.rs         - Syntax parsing
├── ast.rs            - AST node definitions
├── semantic.rs       - Type checking and scope analysis
├── errors.rs         - Diagnostic types (Span, Diagnostic, etc.)
└── linter/
    ├── mod.rs        - Rule trait definition
    └── rules/
        ├── mod.rs
        ├── camel_case.rs      - Variable naming (camelCase)
        ├── constant_case.rs   - Constant naming (SCREAMING_SNAKE_CASE)
        └── line_length.rs     - Max 120 characters per line
```

### Error Handling

**Diagnostic System:**
```rust
pub struct Diagnostic {
    pub kind: DiagnosticKind,  // Lex, Parse, Semantic, Linter
    pub message: String,
    pub span: Span,            // Range<usize> - byte offsets
    pub notes: Vec<String>,
}
```

**Error Display:**
- Converts byte offsets to line:column positions
- Shows source line with caret (^) highlighting
- Includes helpful notes and expected tokens

Example output:
```
parser error: Expected ';' after variable declaration
 --> 3:15-3:16
   3 |     int x = 5
     |               ^
 note: expected: ';'
```

---

## Testing

### Test Structure

```
tests/
├── cli_diagnostics.rs    - Integration tests via CLI
└── test-bal-files/       - Sample Ballerina programs
    ├── hello.bal
    ├── arrays.bal
    ├── functions.bal
    └── ...
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture

# Test specific file
./target/release/blazelint tests/test-bal-files/hello.bal
```

### Current Test Coverage

**Lexer Tests:**
- Unterminated strings, block comments
- Single `&` and `|` characters (should be `&&` or `||`)
- Malformed exponents
- Unexpected characters

**Parser Tests:**
- Missing semicolons, parentheses, braces
- Invalid assignment targets
- `const` with type annotation (should fail)

**Semantic Tests:**
- Type mismatches in assignments
- Final/const reassignment
- Missing return values
- Uninitialized variables

**Linter Tests:**
- Line length (>120 chars)
- camelCase for variables
- SCREAMING_SNAKE_CASE for constants

---

## Development Guide

### Adding a New Token

1. **Define in `lexer.rs`:**
   ```rust
   pub enum Token {
       // ...
       NewKeyword,
   }
   ```

2. **Add to keyword recognition:**
   ```rust
   fn identifier(&mut self) -> Token {
       let text = &self.input[self.start..self.current];
       match text {
           // ...
           "newkeyword" => Token::NewKeyword,
           _ => Token::Identifier(text.to_string()),
       }
   }
   ```

3. **Update `Iterator` impl if it's an operator:**
   ```rust
   fn next(&mut self) -> Option<Self::Item> {
       // ...
       match c {
           '@' => Ok(self.create_token(Token::At)),
           // ...
       }
   }
   ```

### Adding a New AST Node

1. **Define in `ast.rs`:**
   ```rust
   pub enum Stmt {
       // ...
       NewStmt {
           field: Type,
           span: Span,
       },
   }
   ```

2. **Update `span()` method:**
   ```rust
   impl Stmt {
       pub fn span(&self) -> &Span {
           match self {
               // ...
               Stmt::NewStmt { span, .. } => span,
           }
       }
   }
   ```

3. **Add parser method:**
   ```rust
   fn new_stmt(&mut self) -> ParseResult<Stmt> {
       // Parse the statement
   }
   ```

4. **Update `declaration()` or `statement()`:**
   ```rust
   fn declaration(&mut self) -> ParseResult<Stmt> {
       if matches!(self.peek(), Some(Token::NewKeyword)) {
           self.new_stmt()
       } else {
           // ...
       }
   }
   ```

5. **Handle in semantic analyzer:**
   ```rust
   fn check_stmt(&mut self, stmt: &Stmt) {
       match stmt {
           // ...
           Stmt::NewStmt { field, .. } => {
               // Type check the statement
           }
       }
   }
   ```

### Adding a Linter Rule

1. **Create `src/linter/rules/my_rule.rs`:**
   ```rust
   use crate::{ast::Stmt, errors::{Diagnostic, DiagnosticKind}, linter::Rule};
   
   pub struct MyRule;
   
   impl Rule for MyRule {
       fn name(&self) -> &'static str {
           "my-rule"
       }
       
       fn description(&self) -> &'static str {
           "Description of what this rule checks"
       }
       
       fn validate(&self, statement: &Stmt, source: &str) -> Vec<Diagnostic> {
           let mut diagnostics = Vec::new();
           
           // Check the statement and create diagnostics
           
           diagnostics
       }
   }
   ```

2. **Register in `src/linter/rules/mod.rs`:**
   ```rust
   pub mod my_rule;
   ```

3. **Add to rules list in `src/main.rs`:**
   ```rust
   fn run_linter(ast: &[Stmt], source: &str, _line_starts: &[usize]) -> Result<(), Vec<Diagnostic>> {
       let rules: Vec<Box<dyn Rule>> = vec![
           Box::new(CamelCase),
           Box::new(ConstantCase),
           Box::new(LineLength),
           Box::new(MyRule),  // Add here
       ];
       // ...
   }
   ```

---

## Known Issues and TODOs

### Critical for Test Files

1. **Array/Map Type Inference:**
   ```rust
   // TODO in semantic.rs check_expr()
   Expr::ArrayLiteral { elements, .. } => {
       // Infer common element type from elements
   }
   ```

2. **Import Resolution:**
   - Parse and track imports (currently parsed but not resolved)
   - Mark external module functions as valid

3. **Method Type Checking:**
   ```rust
   // TODO: Basic method signature lookup
   Expr::MethodCall { .. } => {
       // Return appropriate type instead of Unknown
   }
   ```

4. **Loop Context Validation:**
   ```rust
   // TODO: Validate break/continue are inside loops
   Stmt::Break { span } | Stmt::Continue { span } => {
       // Track loop depth
   }
   ```

5. **String Template Expression Parsing:**
   - Parse `${}` interpolations as expressions
   - Type check interpolated values

---

## BNF Grammar Reference

The complete grammar is in `BNF.md`. Key constructs:

### Type Descriptors
```bnf
<type_descriptor> ::= <basic_type> <type_suffix>*
                    | "map" "<" <type_descriptor> ">"
<type_suffix> ::= "[" [<array_dimension>] "]"
                | "?"
                | "|" <type_descriptor>
```

### Import Declarations
```bnf
<import_declaration> ::= "import" <package_name> ";"
<package_name> ::= IDENTIFIER ("/" IDENTIFIER)*
```

### Foreach Loops
```bnf
<foreach_statement> ::= "foreach" <type_descriptor> <identifier> "in" <expression> <block>
```

### Expressions
```bnf
<ternary> ::= <logic_or> ("?" <logic_or> ":" <ternary>)?
            | <logic_or> "?:" <logic_or>

<postfix_op> ::= "[" <expression> "]"
               | "." <identifier> "(" [<call_arguments>] ")"
               | "(" [<call_arguments>] ")"
```

---

## Building and Running

### Development Build
```bash
cargo build
./target/debug/blazelint <file.bal>
```

### Release Build
```bash
cargo build --release
./target/release/blazelint <file.bal>
```

### Linting the Linter
```bash
cargo clippy
cargo fmt --check
```

### Documentation
```bash
cargo doc --open
```

---

## Example Usage

### Basic Validation
```bash
$ ./target/release/blazelint tests/test-bal-files/hello.bal
Ballerina Linter (WIP)
--- Input Code ---
import ballerina/io;

public function main() {
    io:println("Hello, World!");
}
----------------------------
[tokens and AST output]
semantic error: Call to unknown function 'io:println'
```

### With Arrays
```ballerina
public function main() {
    int[3] numbers = [1, 2, 3];      // ✓ Parses correctly
    string name = "test";             // ✓ Parses correctly
    final int count = 5;              // ✓ Parses correctly
}
```

### Linter Rules
```ballerina
int snake_case_var = 1;  // ✗ linter error: not camelCase
const lowercase = 1;      // ✗ linter error: not SCREAMING_SNAKE_CASE
string x = "very long line exceeding 120 characters...";  // ✗ line length
```

---

## Project Scope

**Goal:** Research project to validate Rust for linter implementation

**Scope:** Support BNF grammar coverage for test files in `tests/test-bal-files/` without errors

**Non-Goals:**
- Production-ready linter
- Full Ballerina language support
- Advanced tooling (LSP, formatting, refactoring)

---

## Contributing

When adding features:

1. **Update BNF.md first** - Define grammar changes
2. **Add tokens to lexer** - With tests
3. **Extend AST** - Add new node types
4. **Implement parser** - Follow grammar closely
5. **Add semantic checks** - Type validation
6. **Add tests** - In `tests/cli_diagnostics.rs`
7. **Update this document** - Keep notes current

### Code Style
- Follow Rust naming conventions
- Use `rustfmt` for formatting
- Add documentation comments (`///`) for public APIs
- Keep functions focused and small
- Use `Result` for error handling

---

**Last Updated:** October 14, 2025  
**Maintainers:** Check repository contributors
