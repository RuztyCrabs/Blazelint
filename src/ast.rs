#[derive(Debug)]
#[allow(dead_code)]
pub enum Expr {
  Binary(Box<Expr>, BinaryOp, Box<Expr>),
  Unary(UnaryOp, Box<Expr>),
  Literal(Literal),
  Variable(String),
  Grouping(Box<Expr>),
  Assign { name: String, value: Box<Expr> },
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Literal {
  Number(f64),
  String(String),
  Boolean(bool),
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum BinaryOp {
  Plus, Minus, Star, Slash,
  EqualEqual, NotEqual,
  Greater, GreaterEqual,
  Less, LessEqual,
  And, Or,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum UnaryOp {
  Bang, Minus,
}

#[derive(Debug)]
#[allow(dead_code)]
pub enum Stmt {
  VarDecl {
    name: String,
    type_annotation: Option<String>,
    initializer: Option<Expr>,
  },
  Expression(Expr),
  Return(Option<Expr>),
  Panic(Expr),
  If {
    condition: Expr,
    then_branch: Vec<Stmt>,
    else_branch: Option<Vec<Stmt>>,
  },
  Function {
    name: String,
    params: Vec<(String, String)>,
    return_type: Option<String>,
    body: Vec<Stmt>,
  }
}
