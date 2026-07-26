//! Tokeniser for the Blazelint front-end.
//!
//! The lexer converts raw source text into a stream of token triples annotated
//! with byte offsets. Subsequent stages use these spans to highlight precise
//! error locations and to reconstruct lexemes as needed.
use crate::errors::LexError;

/// Tokens recognised by the Ballerina subset Blazelint currently supports.
#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    /// Keywords
    Import,
    Public,
    Final,
    Var,
    Function,
    If,
    Else,
    While,
    Foreach,
    In,
    Return,
    Panic,
    Check,
    Returns,
    Int,
    String,
    Boolean,
    Float,
    Decimal,
    Byte,
    Anydata,
    Map,
    True,
    False,
    Const,
    Break,
    Continue,
    Is,
    Record,
    Object,
    Type,
    Enum,
    Class,
    Configurable,
    Isolated,
    Xmlns,
    New,
    Checkpanic,
    Trap,
    Typeof,
    Let,
    Match,
    Do,
    On,
    Fail,
    Lock,
    Fork,
    Transaction,
    Retry,
    Rollback,
    Commit,
    Worker,
    Wait,

    // Operators
    Plus,
    Minus,
    Star,
    StarStar, // ** (XML descendants step)
    Slash,
    Percent,
    Bang,
    Eq,
    EqEq,
    EqEqEq,
    BangEq,
    BangEqEq,
    Gt,
    Ge,
    Lt,
    Le,
    AmpAmp,
    PipePipe,
    Amp,
    Pipe,
    Caret,
    Tilde,
    LtLt,
    GtGt,
    GtGtGt,
    PlusEq,
    MinusEq,
    Question,
    QuestionColon,
    DotDotDot,
    DotDotLt,     // ..<
    Arrow,        // =>
    RightArrow,   // ->
    RightArrowGt, // ->> (synchronous send)
    LeftArrow,    // <- (worker receive)

    // Delimiters
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Colon,
    Semicolon,
    Comma,
    Dot,
    At, // @ (annotation attachment)

    // Literals
    Number(f64),
    StringLiteral(String),
    StringTemplate(String),
    Identifier(String),
}

/// Streaming lexer that yields `(start, token, end)` triples for each lexeme.
pub struct Lexer<'input> {
    /// Entire source being tokenised.
    input: &'input str,
    /// Iterator used to peek and consume characters.
    chars: std::iter::Peekable<std::str::Chars<'input>>,
    /// Start byte offset of the current lexeme.
    start: usize,
    /// Cursor pointing at the next character to process.
    current: usize,
}

impl<'input> Lexer<'input> {
    /// Creates a lexer positioned at the start of `input`.
    pub fn new(input: &'input str) -> Self {
        Lexer {
            input,
            chars: input.chars().peekable(),
            start: 0,
            current: 0,
        }
    }

    /// Skips whitespace, `//` line comments, and `#` markdown documentation lines.
    fn skip_whitespace_and_comments(&mut self) -> Result<(), LexError> {
        loop {
            if self.is_at_end() {
                return Ok(());
            }

            let c = match self.peek() {
                Some(ch) => *ch,
                None => return Ok(()),
            };

            match c {
                ' ' | '\r' | '\t' | '\n' => {
                    self.advance();
                }
                // Ballerina markdown documentation lines (`# ...`) are treated as
                // comments and skipped to end of line.
                '#' => {
                    while self.peek() != Some(&'\n') && !self.is_at_end() {
                        self.advance();
                    }
                    if self.peek() == Some(&'\n') {
                        self.advance();
                    }
                }
                // Ballerina has only `//` line comments — there is no `/* */`
                // block-comment form (the official compiler rejects `/*` as an
                // invalid token). Treating `/*` as a comment would also swallow
                // the XML all-children navigation step `x/*`.
                '/' if self.peek_next() == Some('/') => {
                    self.advance(); // Consume '/'
                    self.advance(); // Consume second '/'
                    while self.peek() != Some(&'\n') && !self.is_at_end() {
                        self.advance();
                    }
                    if self.peek() == Some(&'\n') {
                        self.advance();
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    /// Scans a string literal, producing a `LexError` for unterminated strings or escapes.
    fn string(&mut self) -> Result<Token, LexError> {
        while self.peek() != Some(&'"') && !self.is_at_end() {
            if self.peek() == Some(&'\\') {
                self.advance();
                if self.is_at_end() {
                    return Err(LexError::new(
                        "Unterminated escape sequence",
                        self.start..self.current,
                    ));
                }
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(LexError::new(
                "Unterminated string literal",
                self.start..self.current,
            ));
        }
        self.advance(); // Consume the closing '""'

        // Extract the string value (exclude surrounding quotes)
        let value = self.input[self.start + 1..self.current - 1].to_string();

        // Simple unescape for \"
        let unescaped_value = value.replace("\\\"", "\"");
        Ok(Token::StringLiteral(unescaped_value))
    }

    /// Scans a string template literal (backtick strings with ${} interpolation).
    fn string_template(&mut self) -> Result<Token, LexError> {
        let mut template_value = String::new();

        while self.peek() != Some(&'`') && !self.is_at_end() {
            if self.peek() == Some(&'$') && self.peek_next() == Some('{') {
                // For now, we'll just capture the template as-is
                // Full interpolation parsing would need expression parsing in the lexer
                template_value.push('$');
                self.advance();
                template_value.push('{');
                self.advance();

                let mut brace_depth = 1;
                while brace_depth > 0 && !self.is_at_end() {
                    let ch = self.advance().unwrap();
                    template_value.push(ch);
                    match ch {
                        '{' => brace_depth += 1,
                        '}' => brace_depth -= 1,
                        _ => {}
                    }
                }
            } else if self.peek() == Some(&'\\') {
                self.advance();
                template_value.push('\\');
                if !self.is_at_end() {
                    let ch = self.advance().unwrap();
                    template_value.push(ch);
                }
            } else {
                let ch = self.advance().unwrap();
                template_value.push(ch);
            }
        }

        if self.is_at_end() {
            return Err(LexError::new(
                "Unterminated string template",
                self.start..self.current,
            ));
        }
        self.advance(); // Consume the closing '`'

        Ok(Token::StringTemplate(template_value))
    }

    /// Scans a numeric literal: hex (`0x1F`), integer, float, float with
    /// exponent, and an optional `d`/`f` type suffix.
    fn number(&mut self) -> Result<Token, LexError> {
        // Hexadecimal literal `0x...` / `0X...`.
        if self.input.as_bytes().get(self.start) == Some(&b'0')
            && matches!(self.peek(), Some(&'x') | Some(&'X'))
        {
            self.advance(); // consume 'x'
            while self.peek().is_some_and(|&c| c.is_ascii_hexdigit()) {
                self.advance();
            }
            let digits = &self.input[self.start + 2..self.current];
            return i64::from_str_radix(digits, 16)
                .map(|v| Token::Number(v as f64))
                .map_err(|e| {
                    LexError::new(
                        format!("Invalid hex literal '0x{digits}': {e}"),
                        self.start..self.current,
                    )
                });
        }

        while self.peek().is_some_and(|&c| c.is_ascii_digit()) {
            self.advance();
        }

        // Look for a fractional part
        if self.peek() == Some(&'.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance(); // Consume '.'
            while self.peek().is_some_and(|&c| c.is_ascii_digit()) {
                self.advance();
            }
        }

        // Look for exponent part
        if self.peek().is_some_and(|&c| c == 'e' || c == 'E') {
            self.advance(); // Consume 'e' or 'E'
            if self.peek().is_some_and(|&c| c == '+' || c == '-') {
                self.advance(); // Consume '+'  or '-'
            }
            if self.peek().is_some_and(|&c| c.is_ascii_digit()) {
                while self.peek().is_some_and(|&c| c.is_ascii_digit()) {
                    self.advance();
                }
            } else {
                return Err(LexError::new(
                    "Malformed exponent in number literal",
                    self.start..self.current,
                ));
            }
        }

        // Capture the numeric text, then consume an optional `d`/`f` suffix
        // (decimal/float), which is not part of the parsed value.
        let value_str = self.input[self.start..self.current].to_string();
        if self
            .peek()
            .is_some_and(|&c| matches!(c, 'd' | 'D' | 'f' | 'F'))
        {
            self.advance();
        }
        value_str.parse::<f64>().map(Token::Number).map_err(|e| {
            LexError::new(
                format!("Invalid number literal '{value_str}': {e}"),
                self.start..self.current,
            )
        })
    }

    /// Scans an identifier or recognises a reserved keyword in the Ballerina subset.
    fn identifier(&mut self) -> Token {
        while self
            .peek()
            .is_some_and(|&c| c.is_ascii_alphanumeric() || c == '_')
        {
            self.advance();
        }

        let text = &self.input[self.start..self.current];
        match text {
            "import" => Token::Import,
            "public" => Token::Public,
            "var" => Token::Var,
            "final" => Token::Final,
            "function" => Token::Function,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "foreach" => Token::Foreach,
            "in" => Token::In,
            "return" => Token::Return,
            "panic" => Token::Panic,
            "check" => Token::Check,
            "returns" => Token::Returns,
            "int" => Token::Int,
            "string" => Token::String,
            "boolean" => Token::Boolean,
            "float" => Token::Float,
            "decimal" => Token::Decimal,
            "byte" => Token::Byte,
            "anydata" => Token::Anydata,
            "map" => Token::Map,
            "true" => Token::True,
            "false" => Token::False,
            "const" => Token::Const,
            "break" => Token::Break,
            "continue" => Token::Continue,
            "is" => Token::Is,
            "record" => Token::Record,
            "object" => Token::Object,
            "type" => Token::Type,
            "enum" => Token::Enum,
            "class" => Token::Class,
            "configurable" => Token::Configurable,
            "isolated" => Token::Isolated,
            "xmlns" => Token::Xmlns,
            "new" => Token::New,
            "checkpanic" => Token::Checkpanic,
            "trap" => Token::Trap,
            "typeof" => Token::Typeof,
            "let" => Token::Let,
            "match" => Token::Match,
            "do" => Token::Do,
            "on" => Token::On,
            "fail" => Token::Fail,
            "lock" => Token::Lock,
            "fork" => Token::Fork,
            "transaction" => Token::Transaction,
            "retry" => Token::Retry,
            "rollback" => Token::Rollback,
            "commit" => Token::Commit,
            "worker" => Token::Worker,
            "wait" => Token::Wait,
            _ => Token::Identifier(text.to_string()),
        }
    }

    //-------------- Helpers ---------------------------

    /// Creates a token triple `[start, token, end)` covering the current lexeme.
    fn create_token(&self, token_type: Token) -> (usize, Token, usize) {
        (self.start, token_type, self.current)
    }

    /// Advances the lexer and consumes the next character, if any.
    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next();
        if let Some(ch) = c {
            self.current += ch.len_utf8();
        }
        c
    }

    /// Peeks at the next character without consuming it.
    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    /// Peeks two characters ahead without moving the cursor.
    fn peek_next(&mut self) -> Option<char> {
        let mut temp_chars = self.chars.clone();
        temp_chars.next(); // Consume the first char
        temp_chars.next() // Peek at the second
    }

    /// Consumes the next character only when it matches `expected`.
    fn match_char(&mut self, expected: char) -> bool {
        if let Some(&c) = self.peek() {
            if c == expected {
                self.advance();
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Returns true once the cursor has consumed the entire input.
    fn is_at_end(&mut self) -> bool {
        self.peek().is_none()
    }
}

/// Implements `Iterator` so the lexer can be used directly in `for` loops.
impl Iterator for Lexer<'_> {
    type Item = Result<(usize, Token, usize), LexError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Skip whitespace and comments before finding the next token
        if let Err(err) = self.skip_whitespace_and_comments() {
            return Some(Err(err));
        }

        // Update start position for the new token after skipping
        self.start = self.current;

        // Check for end of input AFTER skipping
        let c = self.advance()?;

        let result = match c {
            '(' => Ok(self.create_token(Token::LParen)),
            ')' => Ok(self.create_token(Token::RParen)),
            '{' => Ok(self.create_token(Token::LBrace)),
            '}' => Ok(self.create_token(Token::RBrace)),
            '[' => Ok(self.create_token(Token::LBracket)),
            ']' => Ok(self.create_token(Token::RBracket)),
            ':' => Ok(self.create_token(Token::Colon)),
            ';' => Ok(self.create_token(Token::Semicolon)),
            ',' => Ok(self.create_token(Token::Comma)),
            '@' => Ok(self.create_token(Token::At)),
            '.' => {
                if self.match_char('.') {
                    // `...` inclusive range or `..<` half-open range.
                    if self.match_char('.') {
                        Ok(self.create_token(Token::DotDotDot))
                    } else if self.match_char('<') {
                        Ok(self.create_token(Token::DotDotLt))
                    } else {
                        Ok(self.create_token(Token::DotDotDot))
                    }
                } else {
                    Ok(self.create_token(Token::Dot))
                }
            }
            '+' => {
                if self.match_char('=') {
                    Ok(self.create_token(Token::PlusEq))
                } else {
                    Ok(self.create_token(Token::Plus))
                }
            }
            '-' => {
                if self.match_char('=') {
                    Ok(self.create_token(Token::MinusEq))
                } else if self.match_char('>') {
                    if self.match_char('>') {
                        // `->>` synchronous send action.
                        Ok(self.create_token(Token::RightArrowGt))
                    } else {
                        Ok(self.create_token(Token::RightArrow))
                    }
                } else {
                    Ok(self.create_token(Token::Minus))
                }
            }
            '*' => {
                if self.match_char('*') {
                    Ok(self.create_token(Token::StarStar))
                } else {
                    Ok(self.create_token(Token::Star))
                }
            }
            '/' => Ok(self.create_token(Token::Slash)),
            '%' => Ok(self.create_token(Token::Percent)),
            '~' => Ok(self.create_token(Token::Tilde)),
            '^' => Ok(self.create_token(Token::Caret)),
            '?' => {
                if self.match_char(':') {
                    Ok(self.create_token(Token::QuestionColon))
                } else {
                    Ok(self.create_token(Token::Question))
                }
            }
            '!' => {
                if self.match_char('=') {
                    if self.match_char('=') {
                        Ok(self.create_token(Token::BangEqEq))
                    } else {
                        Ok(self.create_token(Token::BangEq))
                    }
                } else {
                    Ok(self.create_token(Token::Bang))
                }
            }
            '=' => {
                if self.match_char('=') {
                    if self.match_char('=') {
                        Ok(self.create_token(Token::EqEqEq))
                    } else {
                        Ok(self.create_token(Token::EqEq))
                    }
                } else if self.match_char('>') {
                    Ok(self.create_token(Token::Arrow))
                } else {
                    Ok(self.create_token(Token::Eq))
                }
            }
            '>' => {
                if self.match_char('>') {
                    if self.match_char('>') {
                        Ok(self.create_token(Token::GtGtGt))
                    } else {
                        Ok(self.create_token(Token::GtGt))
                    }
                } else if self.match_char('=') {
                    Ok(self.create_token(Token::Ge))
                } else {
                    Ok(self.create_token(Token::Gt))
                }
            }
            '<' => {
                if self.match_char('<') {
                    Ok(self.create_token(Token::LtLt))
                } else if self.match_char('=') {
                    Ok(self.create_token(Token::Le))
                } else if self.match_char('-') {
                    Ok(self.create_token(Token::LeftArrow))
                } else {
                    Ok(self.create_token(Token::Lt))
                }
            }
            '&' => {
                if self.match_char('&') {
                    Ok(self.create_token(Token::AmpAmp))
                } else {
                    Ok(self.create_token(Token::Amp))
                }
            }
            '|' => {
                if self.match_char('|') {
                    Ok(self.create_token(Token::PipePipe))
                } else {
                    Ok(self.create_token(Token::Pipe))
                }
            }
            '\'' => {
                // Quoted identifier `'ident` — lets reserved words be used as
                // identifiers. The leading quote is kept in the token text so it
                // stays distinct from the corresponding keyword.
                while self
                    .peek()
                    .is_some_and(|&c| c.is_ascii_alphanumeric() || c == '_')
                {
                    self.advance();
                }
                let text = self.input[self.start..self.current].to_string();
                Ok(self.create_token(Token::Identifier(text)))
            }
            '"' => self.string().map(|t| self.create_token(t)), // Scan string literal
            '`' => self.string_template().map(|t| self.create_token(t)), // Scan string template
            d if d.is_ascii_digit() => self.number().map(|t| self.create_token(t)), // Scan number literal
            a if a.is_ascii_alphabetic() || a == '_' => {
                // Call the mutable method first
                let id_token = self.identifier();
                // Now that the mutable borrow from `self.identifier()` is released
                Ok(self.create_token(id_token))
            }
            _ => Err(LexError::new(
                format!("Unexpected character: '{c}'"),
                self.start..self.current,
            )),
        };

        Some(result)
    }
}
