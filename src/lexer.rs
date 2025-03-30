#[derive(Debug, PartialEq, Clone)]
pub enum TokenType {
    Keyword,      // import, public, function
    Identifier,   // ballerina, io, main, println
    Symbol,       // ;, (, ), {, }, ., :
    StringLiteral,// "Hello, World!"
    Whitespace,   // spaces, tabs, newlines
    Comment,      // Single line or multi-line comments
    Unknown,      // Anything else
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
    pub line: usize,
    pub column: usize,
}

pub fn tokenize(source_code: &str, keywords: &[String]) -> Vec<Token> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = source_code.chars().collect();
    
    let mut line = 1;
    let mut column = 1;
    let mut pos = 0;
    
    while pos < chars.len() {
        match chars[pos] {
            // Handle whitespace
            c if c.is_whitespace() => {
                let (token, new_pos, new_line, new_col) = tokenize_whitespace(&chars, pos, line, column);
                tokens.push(token);
                pos = new_pos;
                line = new_line;
                column = new_col;
            },
            
            // Handle comments
            '/' if pos + 1 < chars.len() && chars[pos + 1] == '/' => {
                let (token, new_pos, new_line, new_col) = tokenize_line_comment(&chars, pos, line, column);
                tokens.push(token);
                pos = new_pos;
                line = new_line;
                column = new_col;
            },
            
            // Handle identifiers and keywords
            c if c.is_alphabetic() || c == '_' => {
                let (token, new_pos, new_col) = tokenize_identifier(&chars, pos, line, column, keywords);
                tokens.push(token);
                pos = new_pos;
                column = new_col;
            },
            
            // Handle string literals
            '"' => {
                let (token, new_pos, new_line, new_col) = tokenize_string(&chars, pos, line, column);
                tokens.push(token);
                pos = new_pos;
                line = new_line;
                column = new_col;
            },
            
            // Handle symbols
            c @ (';' | '(' | ')' | '{' | '}' | '.' | ':') => {
                tokens.push(Token {
                    token_type: TokenType::Symbol,
                    value: c.to_string(),
                    line,
                    column,
                });
                pos += 1;
                column += 1;
            },
            
            // Handle unknown tokens
            _ => {
                tokens.push(Token {
                    token_type: TokenType::Unknown,
                    value: chars[pos].to_string(),
                    line,
                    column,
                });
                pos += 1;
                column += 1;
            },
        }
    }
    
    tokens
}

fn tokenize_whitespace(chars: &[char], pos: usize, line: usize, column: usize) -> (Token, usize, usize, usize) {
    let mut value = String::new();
    let mut current_pos = pos;
    let mut current_line = line;
    let mut current_column = column;
    
    while current_pos < chars.len() && chars[current_pos].is_whitespace() {
        if chars[current_pos] == '\n' {
            current_line += 1;
            current_column = 1;
        } else {
            current_column += 1;
        }
        value.push(chars[current_pos]);
        current_pos += 1;
    }
    
    (
        Token {
            token_type: TokenType::Whitespace,
            value,
            line,
            column,
        },
        current_pos,
        current_line,
        current_column,
    )
}

fn tokenize_line_comment(chars: &[char], pos: usize, line: usize, column: usize) -> (Token, usize, usize, usize) {
    let mut value = String::new();
    let mut current_pos = pos;
    let current_line = line;
    let mut current_column = column;
    
    // Add the // characters
    value.push(chars[current_pos]);
    current_pos += 1;
    current_column += 1;
    value.push(chars[current_pos]);
    current_pos += 1;
    current_column += 1;
    
    // Continue until end of line or end of file
    while current_pos < chars.len() && chars[current_pos] != '\n' {
        value.push(chars[current_pos]);
        current_pos += 1;
        current_column += 1;
    }
    
    (
        Token {
            token_type: TokenType::Comment,
            value,
            line,
            column,
        },
        current_pos,
        current_line,
        current_column,
    )
}

fn tokenize_identifier(chars: &[char], pos: usize, line: usize, column: usize, keywords: &[String]) -> (Token, usize, usize) {
    let mut value = String::new();
    let mut current_pos = pos;
    let mut current_column = column;
    
    while current_pos < chars.len() && 
          (chars[current_pos].is_alphanumeric() || chars[current_pos] == '_' || chars[current_pos] == '/') {
        value.push(chars[current_pos]);
        current_pos += 1;
        current_column += 1;
    }
    
    let token_type = if keywords.contains(&value) {
        TokenType::Keyword
    } else {
        TokenType::Identifier
    };
    
    (
        Token {
            token_type,
            value,
            line,
            column,
        },
        current_pos,
        current_column,
    )
}

fn tokenize_string(chars: &[char], pos: usize, line: usize, column: usize) -> (Token, usize, usize, usize) {
    let mut value = String::new();
    let mut current_pos = pos;
    let mut current_line = line;
    let mut current_column = column;
    
    // Add the opening quote
    value.push(chars[current_pos]);
    current_pos += 1;
    current_column += 1;
    
    let mut escaped = false;
    
    while current_pos < chars.len() {
        let c = chars[current_pos];
        
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if c == '"' {
            // Add the closing quote
            value.push(c);
            current_pos += 1;
            current_column += 1;
            break;
        } else if c == '\n' {
            current_line += 1;
            current_column = 1;
        }
        
        value.push(c);
        current_pos += 1;
        current_column += 1;
    }
    
    (
        Token {
            token_type: TokenType::StringLiteral,
            value,
            line,
            column,
        },
        current_pos,
        current_line,
        current_column,
    )
}
