#[derive(Debug, Clone)]
pub enum Expr {
    Identifier(String),
    Number(i64),
    Binary {
        left: Box<Expr>,
        operator: String,
        right: Box<Expr>,
    },
    StringLiteral(String), // Add this variant to handle string literals
    // Add more as needed (e.g., FunctionCall, Assignment, etc.)
}
