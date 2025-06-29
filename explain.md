# Blazelint: From Source Code to Abstract Syntax Tree (AST)

Blazelint is a linter program designed to analyze source code written in a Ballerina language. Its primary function is to transform raw source code into an Abstract Syntax Tree (AST), which is a tree representation of the abstract syntactic structure of source code. This document explains the process, detailing the roles of the Lexer and Parser components.

## Overall Flow

The process of converting source code into an AST in Blazelint follows a two-main-phase approach:

1.  **Lexical Analysis (Tokenization):** The raw input string is broken down into a stream of meaningful units called "tokens" by the `Lexer`.
2.  **Syntactic Analysis (Parsing):** The stream of tokens is then consumed by the `Parser`, which applies the language's grammar rules to build the hierarchical AST.

```
Source Code (String)
       ↓
     Lexer
       ↓
Tokens (Vec<(usize, Token, usize)>)
       ↓
     Parser
       ↓
Abstract Syntax Tree (Vec<Stmt>)
```

## 1. Lexical Analysis (Lexer)

The `Lexer` (defined in `src/lexer.rs`) is responsible for reading the input source code character by character and grouping them into `Token`s. Each token represents a fundamental building block of the language (e.g., keywords, identifiers, operators, literals).

### `Lexer` Structure

```rust
pub struct Lexer<'input'> {
  input: &'input str,
  chars: std::iter::Peekable<std::str::Chars<'input>>,
  start: usize,   // Start offset of the current lexeme
  current: usize, // Current char being considered
}
```

### Key Functions:

*   **`Lexer::new(input: &'input str) -> Self`**
    *   **Purpose:** Creates a new `Lexer` instance.
    *   **Arguments:** `input` - A string slice of the source code to be tokenized.
    *   **Returns:** A `Lexer` instance.
    *   **How it works:** Initializes the lexer with the input string, setting up an iterator over its characters and resetting internal pointers.

*   **`impl<'input'> Iterator for Lexer<'input'>` (The `next` method)**
    *   **Purpose:** This is the core of the lexer. It implements the `Iterator` trait, allowing the lexer to be iterated over to produce tokens. Each call to `next` attempts to produce the next token.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Option<Result<(usize, Token, usize), String>>` - An `Option` because it returns `None` when the end of the input is reached. The `Result` indicates whether a token was successfully created (`Ok`) or if a lexing error occurred (`Err`). The `(usize, Token, usize)` tuple contains the start byte offset, the `Token` itself, and the end byte offset of the token in the source string.
    *   **How it works:**
        1.  Calls `skip_whitespace_and_comments()` to ignore non-meaningful characters.
        2.  Sets `self.start` to the current position.
        3.  `advance()`s to get the first character of the potential token.
        4.  Uses a `match` statement on the current character to determine the token type (e.g., `(`, `{`, `+`, `=`, `"`, digits, alphabetic characters).
        5.  Delegates to specialized helper functions (`string`, `number`, `identifier`) for more complex token types.
        6.  Constructs and returns the `(start, token, end)` tuple.

### Lexer Helper Functions:

*   **`fn advance(&mut self) -> Option<char>`**
    *   **Purpose:** Consumes the next character from the input and moves the `current` pointer forward.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Option<char>` - The consumed character, or `None` if at the end of the input.

*   **`fn peek(&mut self) -> Option<&char>`**
    *   **Purpose:** Looks at the next character without consuming it.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Option<&char>` - A reference to the next character, or `None` if at the end of the input.

*   **`fn peek_next(&mut self) -> Option<char>`**
    *   **Purpose:** Looks at the character two positions ahead without consuming any.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Option<char>` - The character two positions ahead, or `None`.

*   **`fn match_char(&mut self, expected: char) -> bool`**
    *   **Purpose:** Checks if the next character matches `expected`. If it does, it consumes the character.
    *   **Arguments:** `&mut self`, `expected` - The character to match.
    *   **Returns:** `bool` - `true` if a match occurred and the character was consumed, `false` otherwise.

*   **`fn is_at_end(&mut self) -> bool`**
    *   **Purpose:** Checks if the lexer has reached the end of the input.
    *   **Arguments:** `&mut self`
    *   **Returns:** `bool` - `true` if at the end, `false` otherwise.

*   **`fn create_token(&self, token_type: Token) -> (usize, Token, usize)`**
    *   **Purpose:** Creates the standard token tuple `(start, token_type, end)`.
    *   **Arguments:** `&self`, `token_type` - The `Token` enum variant.
    *   **Returns:** `(usize, Token, usize)` - The token tuple.

*   **`fn skip_whitespace_and_comments(&mut self)`**
    *   **Purpose:** Advances the lexer past whitespace characters (` `, ``, `	`, `
`) and single-line (`//`) or multi-line (`/* ... */`) comments.
    *   **Arguments:** `&mut self`
    *   **Returns:** `()` (no return value)

*   **`fn string(&mut self) -> Result<Token, String>`**
    *   **Purpose:** Scans a string literal (enclosed in `"`). Handles basic escape sequences.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Result<Token, String>` - An `Ok(Token::StringLiteral)` with the string value, or an `Err` message if the string is unterminated.

*   **`fn number(&mut self) -> Result<Token, String>`**
    *   **Purpose:** Scans a number literal (integers, floats, scientific notation).
    *   **Arguments:** `&mut self`
    *   **Returns:** `Result<Token, String>` - An `Ok(Token::Number)` with the parsed `f64` value, or an `Err` message if the number is malformed.

*   **`fn identifier(&mut self) -> Token`**
    *   **Purpose:** Scans an identifier (variable names, function names) or a keyword.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Token` - Either an `Identifier` token or the corresponding keyword token (e.g., `Token::Var`, `Token::Function`).

## 2. Syntactic Analysis (Parser)

The `Parser` (defined in `src/parser.rs`) takes the stream of tokens generated by the `Lexer` and builds the Abstract Syntax Tree (AST). It uses a technique called "recursive descent parsing," where each grammar rule is typically implemented as a function that recursively calls other functions to parse sub-rules.

### `Parser` Structure

```rust
pub struct Parser {
  tokens: Vec<(usize, Token, usize)>,
  current: usize,
}
```

### Key Functions:

*   **`Parser::new(tokens: Vec<(usize, Token, usize)>) -> Self`**
    *   **Purpose:** Creates a new `Parser` instance.
    *   **Arguments:** `tokens` - A `Vec` of tokens (the output from the `Lexer`).
    *   **Returns:** A `Parser` instance.
    *   **How it works:** Stores the token stream and initializes the `current` pointer to the beginning.

*   **`pub fn parse(&mut self) -> Vec<Stmt>`**
    *   **Purpose:** The main entry point for parsing. It iteratively parses declarations until the end of the token stream is reached.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Vec<Stmt>` - A vector of `Stmt` (statements), representing the root of the AST.
    *   **How it works:** Loops, calling `self.declaration()` to parse each top-level statement until `is_at_end()` is true.

### Parser Grammar Rule Functions (Recursive Descent):

These functions correspond to grammar rules and recursively call each other to build expressions and statements.

*   **`fn declaration(&mut self) -> Stmt`**
    *   **Purpose:** Parses a top-level declaration (e.g., variable declaration, function declaration) or a general statement.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Stmt` - A parsed statement.
    *   **How it works:** Peeks at the current token to decide whether to call `var_decl()`, `function()`, or `statement()`.

*   **`fn var_decl(&mut self) -> Stmt`**
    *   **Purpose:** Parses a variable declaration (e.g., `var myVar: int = 10;`).
    *   **Arguments:** `&mut self`
    *   **Returns:** `Stmt::VarDecl`
    *   **How it works:** Consumes `var`, then an identifier for the name, optionally a colon and a type (using `parse_type`), optionally an `=` and an initializer expression, and finally a semicolon.

*   **`fn statement(&mut self) -> Stmt`**
    *   **Purpose:** Parses various types of statements (e.g., `if`, `return`, `panic`, or simple expression statements).
    *   **Arguments:** `&mut self`
    *   **Returns:** `Stmt`
    *   **How it works:** Peeks at the current token to dispatch to `if_statement()`, `return` handling, `panic` handling, or a general `expression` followed by a semicolon.

*   **`fn if_statement(&mut self) -> Stmt`**
    *   **Purpose:** Parses an `if` statement, including an optional `else` block.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Stmt::If`
    *   **How it works:** Consumes `if`, `(`, parses a `condition` expression, consumes `)`, `{`, parses the `then_block` (using `block()`), optionally consumes `else` and another `{` for the `else_block`.

*   **`fn block(&mut self) -> Vec<Stmt>`**
    *   **Purpose:** Parses a block of statements enclosed in curly braces `{}`.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Vec<Stmt>` - A vector of statements within the block.
    *   **How it works:** Iteratively calls `declaration()` until a closing `}` or end of file is encountered.

*   **`fn function(&mut self) -> Stmt`**
    *   **Purpose:** Parses a function declaration.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Stmt::Function`
    *   **How it works:** Consumes `function`, an identifier for the name, `(`, parses `params` (name: type pairs), `)`, optionally `returns` and a `return_type` (using `parse_type`), `{`, and finally the `body` (using `block()`).

*   **`fn expression(&mut self) -> Expr`**
    *   **Purpose:** The entry point for parsing any expression. It delegates to the lowest precedence operator.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Calls `self.assignment()`.

*   **`fn assignment(&mut self) -> Expr`**
    *   **Purpose:** Parses assignment expressions (e.g., `x = 10;`).
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr` (either `Expr::Assign` or the result of `logic_or`).
    *   **How it works:** Parses the left-hand side (which must be a variable), then checks for an `=` token. If found, it recursively calls `assignment()` for the right-hand side to handle right-associativity.

*   **`fn logic_or(&mut self) -> Expr`**
    *   **Purpose:** Parses logical OR (`||`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Parses a `logic_and()` expression, then iteratively checks for `||` tokens, consuming them and parsing subsequent `logic_and()` expressions to build a binary `Or` expression.

*   **`fn logic_and(&mut self) -> Expr`**
    *   **Purpose:** Parses logical AND (`&&`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Parses an `equality()` expression, then iteratively checks for `&&` tokens, consuming them and parsing subsequent `equality()` expressions to build a binary `And` expression.

*   **`fn equality(&mut self) -> Expr`**
    *   **Purpose:** Parses equality (`==`, `!=`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Parses a `comparison()` expression, then iteratively checks for `==` or `!=` tokens, consuming them and parsing subsequent `comparison()` expressions.

*   **`fn comparison(&mut self) -> Expr`**
    *   **Purpose:** Parses comparison (`>`, `>=`, `<`, `<=`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Parses a `term()` expression, then iteratively checks for comparison operators, consuming them and parsing subsequent `term()` expressions.

*   **`fn term(&mut self) -> Expr`**
    *   **Purpose:** Parses addition (`+`) and subtraction (`-`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Parses a `factor()` expression, then iteratively checks for `+` or `-` tokens, consuming them and parsing subsequent `factor()` expressions.

*   **`fn factor(&mut self) -> Expr`**
    *   **Purpose:** Parses multiplication (`*`) and division (`/`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Parses a `unary()` expression, then iteratively checks for `*` or `/` tokens, consuming them and parsing subsequent `unary()` expressions.

*   **`fn unary(&mut self) -> Expr`**
    *   **Purpose:** Parses unary (`!`, `-`) expressions.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Checks for `!` or `-`. If found, consumes the operator and recursively calls `unary()` to parse the operand. Otherwise, delegates to `primary()`.n*   **`fn primary(&mut self) -> Expr`**
    *   **Purpose:** Parses the most basic expressions: literals (numbers, strings, booleans), identifiers, and grouped expressions `( ... )`.
    *   **Arguments:** `&mut self`
    *   **Returns:** `Expr`
    *   **How it works:** Consumes the next token and matches it to a literal, identifier, or an opening parenthesis. For grouped expressions, it recursively calls `expression()` and consumes the closing parenthesis.

*   **`fn parse_type(&mut self) -> String`**
    *   **Purpose:** Parses a type annotation (e.g., `int`, `string`, `float`, `boolean`, or a custom identifier type).
    *   **Arguments:** `&mut self`
    *   **Returns:** `String` - The string representation of the parsed type.
    *   **How it works:** Consumes the next token and maps it to a type string.

### Parser Helper Functions:

*   **`fn is_at_end(&self) -> bool`**
    *   **Purpose:** Checks if the parser has consumed all tokens.
    *   **Arguments:** `&self`
    *   **Returns:** `bool`

*   **`fn peek(&self) -> &Token`**
    *   **Purpose:** Returns a reference to the current token without consuming it.
    *   **Arguments:** `&self`
    *   **Returns:** `&Token`

*   **`fn previous(&self) -> &Token`**
    *   **Purpose:** Returns a reference to the previously consumed token.
    *   **Arguments:** `&self`
    *   **Returns:** `&Token`

*   **`fn advance(&mut self) -> &Token`**
    *   **Purpose:** Consumes the current token and moves to the next one.
    *   **Arguments:** `&mut self`
    *   **Returns:** `&Token` - A reference to the token that was just consumed.

*   **`fn check(&self, expected: &Token) -> bool`**
    *   **Purpose:** Checks if the current token matches the `expected` token type without consuming it.
    *   **Arguments:** `&self`, `expected` - A reference to the `Token` to match.
    *   **Returns:** `bool`

*   **`fn match_token(&mut self, types: &[Token]) -> bool`**
    *   **Purpose:** Checks if the current token matches any of the `types` provided. If a match is found, the token is consumed.
    *   **Arguments:** `&mut self`, `types` - A slice of `Token` types to match against.
    *   **Returns:** `bool` - `true` if a match occurred and the token was consumed, `false` otherwise.

*   **`fn consume(&mut self, expected: Token, msg: &str)`**
    *   **Purpose:** Asserts that the current token is of the `expected` type and consumes it. If not, it panics with the given error message.
    *   **Arguments:** `&mut self`, `expected` - The `Token` type expected, `msg` - The error message to display on panic.
    *   **Returns:** `()` (no return value)

## 3. Abstract Syntax Tree (AST) Structure

The AST is defined by the `Expr` (expressions) and `Stmt` (statements) enums in `src/ast.rs`. These enums represent the hierarchical structure of the parsed code.

### `Expr` Enum

Represents different types of expressions in the language:

```rust
#[derive(Debug)]
#[allow(dead_code)] // Suppressed for demonstration; in a real project, these would be used.
pub enum Expr {
  Binary(Box<Expr>, BinaryOp, Box<Expr>), // e.g., a + b, x == y
  Unary(UnaryOp, Box<Expr>),             // e.g., !true, -5
  Literal(Literal),                       // e.g., 123, "hello", true
  Variable(String),                       // e.g., myVar
  Grouping(Box<Expr>),
  Assign { name: String, value: Box<Expr> }, // e.g., x = 10
}
```

### `Stmt` Enum

Represents different types of statements in the language:

```rust
#[derive(Debug)]
#[allow(dead_code)] // Suppressed for demonstration; in a real project, these would be used.
pub enum Stmt {
  VarDecl {
    name: String,
    type_annotation: Option<String>,
    initializer: Option<Expr>,
  }, // e.g., var x: int = 10;
  Expression(Expr), // e.g., 1 + 2;
  Return(Option<Expr>), // e.g., return; or return value;
  Panic(Expr),      // e.g., panic "Error!";
  If {
    condition: Expr,
    then_branch: Vec<Stmt>,
    else_branch: Option<Vec<Stmt>>,
  }, // e.g., if (cond) { ... } else { ... }
  Function {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
  }, // e.g., function myFunc(a: int) returns int { ... }
}
```

## Conclusion

By combining the `Lexer`'s ability to break down raw text into tokens and the `Parser`'s recursive descent logic to build a structured AST, Blazelint effectively transforms human-readable code into a machine-understandable representation. This AST can then be used for various purposes, such as static analysis, interpretation, or code generation.

```
