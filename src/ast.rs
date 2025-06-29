/// Represents an expression in the abstract syntax tree.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Expr {
  /// A binary expression with a left operand, an operator, and a right operand.
  Binary(Box<Expr>, BinaryOp, Box<Expr>),
  /// A unary expression with an operator and a single operand.
  Unary(UnaryOp, Box<Expr>),
  /// A literal value (number, string, or boolean).
  Literal(Literal),
  /// A variable reference.
  Variable(String),
  /// A grouped expression, typically enclosed in parentheses.
  Grouping(Box<Expr>),
  /// An assignment expression, assigning a value to a variable.
  Assign { name: String, value: Box<Expr> },
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
  Plus, Minus, Star, Slash,
  EqualEqual, NotEqual,
  Greater, GreaterEqual,
  Less, LessEqual,
  And, Or,
}

/// Represents a unary operator.
#[derive(Debug)]
#[allow(dead_code)]
pub enum UnaryOp {
  Bang, Minus,
}

/// Represents a statement in the abstract syntax tree.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Stmt {
  /// A variable declaration statement.
  VarDecl {
    name: String,
    type_annotation: Option<String>,
    initializer: Option<Expr>,
  },
  /// An expression statement.
  Expression(Expr),
  /// A return statement, optionally with a return value.
  Return(Option<Expr>),
  /// A panic statement, causing an error.
  Panic(Expr),
  /// An if-else statement.
  If {
    condition: Expr,
    then_branch: Vec<Stmt>,
    else_branch: Option<Vec<Stmt>>,
  },
  /// A function declaration statement.
  Function {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
  }
}