use crate::lexer::{Token, TokenType};
use crate::config::LinterConfig;
use crate::parser::Parser;
use crate::ast::Expr;

#[derive(Debug)]
#[allow(dead_code)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug)]
pub struct Diagnostic {
    pub message: String,
    pub severity: Severity,
    pub line: usize,
    pub column: usize,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let severity_str = match self.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARNING",
            Severity::Info => "INFO",
        };
        
        write!(
            f,
            "[{}] at line {}, column {}: {}",
            severity_str, self.line, self.column, self.message
        )
    }
}

pub trait Rule {
    fn check(&self, tokens: &[Token]) -> Vec<Diagnostic>;
    #[allow(dead_code)]
    fn name(&self) -> &'static str;
}


// AST-based Rule
pub trait AstRule {
    fn check(&self, ast: &Expr) -> Vec<Diagnostic>;
    fn name(&self) -> &'static str;
}

// Rule to check for unknown tokens
pub struct UnknownTokenRule;

impl Rule for UnknownTokenRule {
    fn check(&self, tokens: &[Token]) -> Vec<Diagnostic> {
        tokens
            .iter()
            .filter(|token| token.token_type == TokenType::Unknown)
            .map(|token| Diagnostic {
                message: format!("Unknown token: '{}'", token.value),
                severity: Severity::Error,
                line: token.line,
                column: token.column,
            })
            .collect()
    }
    
    fn name(&self) -> &'static str {
        "unknown-token"
    }
}

// Rule to check for proper function declaration
pub struct FunctionDeclarationRule;

impl Rule for FunctionDeclarationRule {
    fn check(&self, tokens: &[Token]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        for (i, token) in tokens.iter().enumerate() {
            if token.token_type == TokenType::Keyword && token.value == "function" {
                // Check if the next non-whitespace token is an identifier
                let mut next_index = i + 1;
                
                while next_index < tokens.len() && tokens[next_index].token_type == TokenType::Whitespace {
                    next_index += 1;
                }
                
                if next_index >= tokens.len() || tokens[next_index].token_type != TokenType::Identifier {
                    diagnostics.push(Diagnostic {
                        message: "Function declaration must be followed by an identifier".to_string(),
                        severity: Severity::Error,
                        line: token.line,
                        column: token.column,
                    });
                }
            }
        }
        
        diagnostics
    }
    
    fn name(&self) -> &'static str {
        "function-declaration"
    }
}

// Rule to check for proper import statements
pub struct ImportStatementRule;

impl Rule for ImportStatementRule {
    fn check(&self, tokens: &[Token]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        for (i, token) in tokens.iter().enumerate() {
            if token.token_type == TokenType::Keyword && token.value == "import" {
                // Check if next non-whitespace token is an identifier
                let mut next_index = i + 1;
                
                while next_index < tokens.len() && tokens[next_index].token_type == TokenType::Whitespace {
                    next_index += 1;
                }
                
                if next_index >= tokens.len() || tokens[next_index].token_type != TokenType::Identifier {
                    diagnostics.push(Diagnostic {
                        message: "Import statement must be followed by a package path".to_string(),
                        severity: Severity::Error,
                        line: token.line,
                        column: token.column,
                    });
                    continue;
                }
                
                // Check for semicolon at the end of import statement
                next_index = i + 1;
                let mut found_semicolon = false;
                
                while next_index < tokens.len() {
                    if tokens[next_index].token_type == TokenType::Symbol && tokens[next_index].value == ";" {
                        found_semicolon = true;
                        break;
                    }
                    if tokens[next_index].token_type == TokenType::Keyword && 
                       (tokens[next_index].value == "import" || tokens[next_index].value == "function") {
                        break;
                    }
                    next_index += 1;
                }
                
                if !found_semicolon {
                    diagnostics.push(Diagnostic {
                        message: "Import statement must end with a semicolon".to_string(),
                        severity: Severity::Error,
                        line: token.line,
                        column: token.column,
                    });
                }
            }
        }
        
        diagnostics
    }
    
    fn name(&self) -> &'static str {
        "import-statement"
    }
}


// AST Rule Example
pub struct DummyAstRule;

impl AstRule for DummyAstRule {
    fn check(&self, ast: &Expr) -> Vec<Diagnostic> {
        // Example rule: just print that AST was analyzed
        println!("Running dummy AST rule on: {:?}", ast);
        vec![]
    }

    fn name(&self) -> &'static str {
        "dummy-ast-rule"
    }
}

pub struct Linter {
    rules: Vec<Box<dyn Rule>>,
    ast_rules: Vec<Box<dyn AstRule>>, 
}

impl Linter {
    pub fn new() -> Self {
        Linter { 
            rules: Vec::new(), 
            ast_rules: Vec::new(),
        }
    }
    
    pub fn add_rule(&mut self, rule: Box<dyn Rule>) {
        self.rules.push(rule);
    }

    pub fn add_ast_rule(&mut self, rule: Box<dyn AstRule>) {
        self.ast_rules.push(rule);
    }
    
    pub fn lint(&self, tokens: &[Token]) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        
        for rule in &self.rules {
            let rule_diagnostics = rule.check(tokens);
            diagnostics.extend(rule_diagnostics);
        }

         // Parse the AST once for all AST-based rules
         let mut parser = Parser::new(tokens.to_vec());
         if let Some(ast) = parser.parse_expression() {
             for ast_rule in &self.ast_rules {
                 diagnostics.extend(ast_rule.check(&ast));
             }
         } else {
             eprintln!("Failed to generate AST.");
         }
        
        diagnostics
    }
}

pub fn lint_tokens(tokens: &[Token], config: &LinterConfig) -> Vec<Diagnostic> {
    let mut linter = Linter::new();
    
    // Add rules based on config
    if config.is_rule_enabled("unknown-token") {
        linter.add_rule(Box::new(UnknownTokenRule));
    }
    
    if config.is_rule_enabled("function-declaration") {
        linter.add_rule(Box::new(FunctionDeclarationRule));
    }
    
    if config.is_rule_enabled("import-statement") {
        linter.add_rule(Box::new(ImportStatementRule));
    }

    if config.is_rule_enabled("dummy-ast-rule") {
        linter.add_ast_rule(Box::new(DummyAstRule)); 
    }
    
    linter.lint(tokens)
}