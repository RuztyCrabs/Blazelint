use crate::errors::Span;

/// Represents an expression in the abstract syntax tree with precise source span information.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Expr {
    /// A binary expression with a left operand, an operator, and a right operand.
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    /// A unary expression with an operator and a single operand.
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    /// A literal value (number, string, or boolean).
    Literal { value: Literal, span: Span },
    /// A variable reference.
    Variable { name: String, span: Span },
    /// A grouped expression, typically enclosed in parentheses.
    Grouping { expression: Box<Expr>, span: Span },
    /// A function or constructor call expression.
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        span: Span,
    },
    /// An assignment expression, assigning a value to a variable.
    Assign {
        name: String,
        value: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    /// Returns the span covering the entire expression.
    pub fn span(&self) -> &Span {
        match self {
            Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Literal { span, .. }
            | Expr::Variable { span, .. }
            | Expr::Grouping { span, .. }
            | Expr::Call { span, .. }
            | Expr::Assign { span, .. } => span,
        }
    }
}

/// Represents a literal value.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Literal {
    /// A floating-point number.
    Number(f64),
    /// A string literal.
    String(String),
    /// A boolean literal (true or false).
    Boolean(bool),
}

/// Represents a binary operator.
#[derive(Debug)]
#[allow(dead_code)]
pub enum BinaryOp {
    Plus,
    Minus,
    Star,
    Slash,
    EqualEqual,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

/// Represents a unary operator.
#[derive(Debug)]
#[allow(dead_code)]
pub enum UnaryOp {
    Bang,
    Minus,
}

/// Represents a statement in the abstract syntax tree.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Stmt {
    /// A variable declaration statement.
    VarDecl {
        is_final: bool,
        name: String,
        name_span: Span,
        type_annotation: Option<String>,
        initializer: Option<Expr>,
        span: Span,
    },
    ConstDecl {
        name: String,
        name_span: Span,
        type_annotation: Option<String>,
        initializer: Expr,
        span: Span,
    },
    /// An expression statement.
    Expression { expression: Expr, span: Span },
    /// A return statement, optionally with a return value.
    Return { value: Option<Expr>, span: Span },
    /// A panic statement, causing an error.
    Panic { value: Expr, span: Span },
    /// An if-else statement.
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        span: Span,
    },
    /// A function declaration statement.
    Function {
        name: String,
        name_span: Span,
        params: Vec<(String, String)>,
        return_type: Option<String>,
        body: Vec<Stmt>,
        span: Span,
    },
}

impl Stmt {
    /// Returns the enclosing span of the statement.
    #[allow(dead_code)]
    pub fn span(&self) -> &Span {
        match self {
            Stmt::VarDecl { span, .. }
            | Stmt::ConstDecl { span, .. }
            | Stmt::Expression { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Panic { span, .. }
            | Stmt::If { span, .. }
            | Stmt::Function { span, .. } => span,
        }
    }
}
