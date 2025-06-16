# Planned BNF for the language

We are considering a small subset of ballerina lang for this.

```
program       → declaration* EOF ;

declaration   → variableDecl
              | functionDecl
              | statement ;

variableDecl  → "var" IDENTIFIER typeAnnotation? initializer? ";" ;
typeAnnotation → ":" type ;
type          → "int" | "string" | "boolean" | "float" ;

initializer   → "=" expression ;

functionDecl  → "function" IDENTIFIER "(" parameters? ")" returnType? block ;
parameters    → parameter ( "," parameter )* ;
parameter     → IDENTIFIER typeAnnotation ;
returnType    → returns type ;
returns       → "returns" ;

statement     → block
              | ifStatement
              | whileStatement
              | forStatement
              | returnStatement
              | panicStatement
              | exprStatement ;

block         → "{" declaration* "}" ;

ifStatement   → "if" "(" expression ")" block ( "else" block )? ;
whileStatement→ "while" "(" expression ")" block ;
forStatement  → "foreach" "(" IDENTIFIER "in" expression ")" block ;
returnStatement → "return" expression? ";" ;
panicStatement  → "panic" expression ";" ;
exprStatement → expression ";" ;

expression    → assignment ;
assignment    → IDENTIFIER "=" assignment
              | logicalOr ;

logicalOr     → logicalAnd ( "||" logicalAnd )* ;
logicalAnd    → equality ( "&&" equality )* ;
equality      → comparison ( ( "==" | "!=" ) comparison )* ;
comparison    → term ( ( ">" | ">=" | "<" | "<=" ) term )* ;
term          → factor ( ( "+" | "-" ) factor )* ;
factor        → unary ( ( "*" | "/" ) unary )* ;
unary         → ( "!" | "-" | "check" ) unary | primary ;

primary       → NUMBER | STRING | BOOLEAN | IDENTIFIER
              | "(" expression ")" | functionCall ;

functionCall  → IDENTIFIER "(" arguments? ")" ;
arguments     → expression ( "," expression )* ;

BOOLEAN       → "true" | "false" ;
```

# File Structure

blazelint/
├── Cargo.toml
└── src/
    ├── main.rs
    ├── lexer.rs
    ├── token.rs
    ├── parser.rs
    ├── ast.rs
    ├── linter.rs
    └── error.rs

| File | Responsibility
| main.rs | Entry point
| token.rs | Token definitions
| lexer.rs | Lexer
| parser.rs	| Parser
| ast.rs | AST nodes
| linter.rs | Lint rules
| error.rs | Error reporting
