# Blazelint Quick Reference

## Quick Start Commands

```bash
# Build
cargo build --release

# Run on file
./target/release/blazelint path/to/file.bal

# Run tests
cargo test

# Run specific test
cargo test test_name

# Format code
cargo fmt

# Lint code
cargo clippy

# Generate docs
cargo doc --open
```

## Common Development Tasks

### Add a New Keyword

1. **In `src/lexer.rs`** - Add to `Token` enum:
   ```rust
   pub enum Token {
       NewKeyword,  // Add here
   ```

2. **In `identifier()` function**:
   ```rust
   match text {
       "newkeyword" => Token::NewKeyword,
   ```

### Add a New Statement Type

1. **In `src/ast.rs`** - Add to `Stmt` enum
2. **In `src/parser.rs`** - Add parsing function
3. **In `src/semantic.rs`** - Add in `check_stmt()` match
4. **Update `span()` method** in both files

### Add a Linter Rule

1. Create `src/linter/rules/my_rule.rs`
2. Add `pub mod my_rule;` to `src/linter/rules/mod.rs`
3. Register in `run_linter()` in `src/main.rs`

## File Structure Quick Map

```
src/
├── main.rs          → Pipeline: lex → parse → semantic → lint
├── lexer.rs         → String → Tokens
├── parser.rs        → Tokens → AST
├── ast.rs           → Node definitions
├── semantic.rs      → Type checking
├── errors.rs        → Diagnostic types
└── linter/
    ├── mod.rs       → Rule trait
    └── rules/       → Individual rules
```

## Key Types

### Span
```rust
pub type Span = Range<usize>;  // Byte offsets in source
```

### Token Triple
```rust
(usize, Token, usize)  // (start_offset, token, end_offset)
```

### Type Descriptor
```rust
TypeDescriptor::Basic("int")
TypeDescriptor::Array { element_type, dimension }
TypeDescriptor::Map { value_type }
TypeDescriptor::Optional(inner)
TypeDescriptor::Union(types)
```

### Diagnostic
```rust
Diagnostic {
    kind: DiagnosticKind,  // Lex, Parse, Semantic, Linter
    message: String,
    span: Span,
    notes: Vec<String>,
}
```

## Common Patterns

### Parse with Lookahead
```rust
if self.match_token(&[Token::Keyword])? {
    // consume and handle
}
```

### Create Diagnostic
```rust
self.report(
    span.clone(),
    format!("Error message: {}", detail)
);
```

### Type Checking
```rust
let expr_type = self.check_expr(expr);
if !Self::can_assign(&expected_type, &expr_type) {
    self.report(span, "Type mismatch");
}
```

## Testing Patterns

### Add Integration Test
In `tests/cli_diagnostics.rs`:
```rust
#[test]
fn test_my_feature() {
    let output = run_cli("code here");
    assert!(output.status.success());
    let out = stdout(&output);
    assert!(out.contains("expected output"));
}
```

### Test Files
Add `.bal` files to `tests/test-bal-files/` for manual testing

## Debugging Tips

### Enable Debug Output
```rust
println!("Debug: {:?}", variable);  // In code
eprintln!("Stderr: {:?}", variable); // To stderr
```

### Pretty Print AST
```rust
println!("{:#?}", ast);  // In main.rs
```

### Check Token Stream
Output is automatically shown in `--- Tokens ---` section

## Error Handling

### Lexer
```rust
return Err(LexError::new("message", span));
```

### Parser
```rust
return Err(ParseError::new("message", span, Some("expected")));
```

### Semantic
```rust
self.report(span, format!("message"));
```

### Linter
```rust
diagnostics.push(Diagnostic::new(
    DiagnosticKind::Linter,
    "message",
    span,
));
```

## Current Limitations

* **TODO for Test Files:**
    - Import resolution (external modules)
    - Array/map type inference
    - Method signature lookup  
    - Loop context for break/continue
    - String template interpolation

* **Working:**
    - Lexing (all tokens)
    - Parsing (all constructs)
    - Basic type checking
    - Linter rules (camelCase, constantCase, lineLength)

---

**Project Scope:** Research to validate Rust for linter implementation. Goal is BNF coverage for test files without errors. Not a production linter.

## Supported Syntax Examples

### Declarations
```ballerina
import ballerina/io;
int x = 5;
int[3] arr = [1, 2, 3];
map<string> m = {key: "value"};
int? optional = ();
final int constant = 10;
```

### Functions
```ballerina
public function main() { }
function add(x: int, y: int) returns int { }
```

### Control Flow
```ballerina
if (x > 0) { }
while (condition) { }
foreach int item in items { }
break;
continue;
```

### Expressions
```ballerina
arr[0]              // Array access
obj.method()        // Method call
io:println()        // Qualified call
x ? y : z           // Ternary
x ?: y              // Elvis
1...10              // Range
<int>value          // Cast
```

## Performance Notes

- Lexer is single-pass
- Parser is recursive descent (no backtracking)
- Semantic analysis is multi-pass (collect functions, then check)
- Each stage can fail independently

## Version Info

- **Rust Edition:** 2021
- **MSRV:** Check `Cargo.toml`
- **Dependencies:** 
  - `assert_cmd` (testing)
  - `tempfile` (testing)
  - Standard library only for main code

---

For detailed explanations, see `IMPLEMENTATION_NOTES.md`
