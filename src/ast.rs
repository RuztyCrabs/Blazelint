use crate::errors::Span;

/// Represents a type descriptor in the Ballerina language.
#[derive(Debug, Clone, PartialEq)]
pub enum TypeDescriptor {
    Basic(String),
    Array {
        element_type: Box<TypeDescriptor>,
        dimension: Option<ArrayDimension>,
    },
    Map {
        value_type: Box<TypeDescriptor>,
    },
    Optional(Box<TypeDescriptor>),
    Union(Vec<TypeDescriptor>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayDimension {
    Fixed(usize),
    Inferred, // for [*]
    Open,     // for []
}

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
    /// Member access expression (e.g., array[0] or obj.field).
    MemberAccess {
        object: Box<Expr>,
        member: Box<Expr>,
        span: Span,
    },
    /// Method call expression (e.g., obj.method()).
    MethodCall {
        object: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
        span: Span,
    },
    /// Array literal expression.
    ArrayLiteral { elements: Vec<Expr>, span: Span },
    /// Map literal expression.
    MapLiteral {
        entries: Vec<(String, Expr)>,
        span: Span,
    },
    /// Ternary conditional expression (condition ? true_expr : false_expr).
    Ternary {
        condition: Box<Expr>,
        true_expr: Box<Expr>,
        false_expr: Box<Expr>,
        span: Span,
    },
    /// Elvis operator (expr ?: default).
    Elvis {
        expr: Box<Expr>,
        default: Box<Expr>,
        span: Span,
    },
    /// Range expression (start...end).
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },
    /// Cast expression (<type>expr).
    Cast {
        type_desc: TypeDescriptor,
        expr: Box<Expr>,
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
            | Expr::Assign { span, .. }
            | Expr::MemberAccess { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::ArrayLiteral { span, .. }
            | Expr::MapLiteral { span, .. }
            | Expr::Ternary { span, .. }
            | Expr::Elvis { span, .. }
            | Expr::Range { span, .. }
            | Expr::Cast { span, .. } => span,
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
    /// Nil literal ().
    Nil,
}

/// Represents a binary operator.
#[derive(Debug)]
#[allow(dead_code)]
pub enum BinaryOp {
    // Arithmetic
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Comparison
    EqualEqual,
    NotEqual,
    EqualEqualEqual,
    NotEqualEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Is,

    // Logical
    And,
    Or,

    // Bitwise
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,

    // Shift
    LeftShift,
    RightShift,
    UnsignedRightShift,

    // Assignment
    PlusAssign,
    MinusAssign,
}

/// Represents a unary operator.
#[derive(Debug)]
#[allow(dead_code)]
pub enum UnaryOp {
    Bang,
    Minus,
    Plus,
    BitwiseNot,
}

/// Represents a statement in the abstract syntax tree.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Stmt {
    /// Import declaration.
    Import {
        package_path: Vec<String>,
        span: Span,
    },
    /// A variable declaration statement.
    VarDecl {
        is_final: bool,
        name: String,
        name_span: Span,
        type_annotation: Option<TypeDescriptor>,
        initializer: Option<Expr>,
        span: Span,
    },
    ConstDecl {
        name: String,
        name_span: Span,
        type_annotation: Option<TypeDescriptor>,
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
    /// A while loop statement.
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// A foreach loop statement.
    Foreach {
        type_annotation: Option<TypeDescriptor>,
        variable: String,
        iterable: Expr,
        body: Vec<Stmt>,
        span: Span,
    },
    /// A break statement.
    Break { span: Span },
    /// A continue statement.
    Continue { span: Span },
    /// A function declaration statement.
    Function {
        is_public: bool,
        name: String,
        name_span: Span,
        params: Vec<(String, TypeDescriptor)>,
        return_type: Option<TypeDescriptor>,
        body: Vec<Stmt>,
        span: Span,
    },
}

impl Stmt {
    /// Returns the enclosing span of the statement.
    #[allow(dead_code)]
    pub fn span(&self) -> &Span {
        match self {
            Stmt::Import { span, .. }
            | Stmt::VarDecl { span, .. }
            | Stmt::ConstDecl { span, .. }
            | Stmt::Expression { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Panic { span, .. }
            | Stmt::If { span, .. }
            | Stmt::While { span, .. }
            | Stmt::Foreach { span, .. }
            | Stmt::Break { span, .. }
            | Stmt::Continue { span, .. }
            | Stmt::Function { span, .. } => span,
        }
    }
}
