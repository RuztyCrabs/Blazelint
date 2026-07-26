use crate::errors::Span;

/// Represents a type descriptor in the Ballerina language.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
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
    /// Tuple type `[T1, T2, ...R]` with an optional trailing rest element.
    Tuple {
        members: Vec<TypeDescriptor>,
        rest: Option<Box<TypeDescriptor>>,
    },
    /// Inline record type `record { ... }` or closed `record {| ... |}`.
    Record {
        fields: Vec<RecordField>,
        rest: Option<Box<TypeDescriptor>>,
        closed: bool,
    },
    /// Object type `object { ... }`. The body is currently parsed leniently and
    /// not retained (parse-tolerant); it exists so type positions accept objects.
    Object,
    /// Function type `function (params) returns T`.
    Function {
        params: Vec<TypeDescriptor>,
        return_type: Option<Box<TypeDescriptor>>,
    },
    /// Generic/parameterised named type: `error<D>`, `future<T>`, `stream<T,C>`,
    /// `typedesc<T>`, `table<R>`, etc.
    Generic {
        name: String,
        args: Vec<TypeDescriptor>,
    },
    /// Intersection type `T1 & T2` (e.g. `readonly & Config`).
    Intersection(Vec<TypeDescriptor>),
    /// Singleton (literal) type such as `1`, `"OPEN"`, or `true`. Stored as the
    /// rendered literal text.
    Singleton(String),
    /// `distinct T` type.
    Distinct(Box<TypeDescriptor>),
    /// Module-qualified type reference `module:Type`.
    Qualified {
        module: String,
        name: String,
    },
}

/// A single field within an inline record type descriptor.
#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub struct RecordField {
    pub name: String,
    pub type_desc: TypeDescriptor,
    pub optional: bool,
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
    /// Member/index access expression (e.g., array[0], map[key]).
    MemberAccess {
        object: Box<Expr>,
        member: Box<Expr>,
        span: Span,
    },
    /// Field access expression (e.g., obj.field, self.count). The field is a
    /// name, not an expression, so it is stored directly rather than resolved as
    /// a variable.
    FieldAccess {
        object: Box<Expr>,
        field: String,
        span: Span,
    },
    /// Assignment to a non-variable lvalue (field or index target), e.g.
    /// `self.count = 1` or `arr[i] = x`. Simple-variable assignment uses `Assign`.
    MemberAssign {
        target: Box<Expr>,
        value: Box<Expr>,
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
    /// Object construction: `new`, `new T(args)`, or `new (args)`.
    New {
        type_desc: Option<TypeDescriptor>,
        arguments: Vec<Expr>,
        span: Span,
    },
    /// A named argument in a call: `f(name = value)`.
    ///
    /// This is deliberately distinct from `Assign`: the name identifies a
    /// parameter of the callee, not a variable in the enclosing scope, so it
    /// must not be resolved against the scope stack.
    NamedArg {
        name: String,
        name_span: Span,
        value: Box<Expr>,
        span: Span,
    },
    /// Check/error-handling prefix expression: `check e`, `checkpanic e`,
    /// `trap e`. The keyword is retained for later semantic differentiation.
    Check {
        keyword: String,
        expr: Box<Expr>,
        span: Span,
    },
    /// `typeof e` expression.
    TypeOf { expr: Box<Expr>, span: Span },
    /// A string/xml/raw template with parsed `${...}` interpolation expressions.
    StringTemplate {
        interpolations: Vec<Expr>,
        span: Span,
    },
    /// Type-test expression: `e is T`.
    TypeTest {
        expr: Box<Expr>,
        ty: TypeDescriptor,
        span: Span,
    },
    /// Let expression: `let <bindings> in <body>`.
    Let {
        bindings: Vec<LetBinding>,
        body: Box<Expr>,
        span: Span,
    },
    /// Anonymous function: `function (params) [returns T] { body }`.
    AnonFunction {
        params: Vec<(String, TypeDescriptor)>,
        return_type: Option<TypeDescriptor>,
        body: Vec<Stmt>,
        span: Span,
    },
    /// Arrow function: `x => e` or `(a, b) => e`.
    Arrow {
        params: Vec<String>,
        body: Box<Expr>,
        span: Span,
    },
    /// Remote method call: `client->method(args)`.
    RemoteCall {
        object: Box<Expr>,
        method: String,
        arguments: Vec<Expr>,
        span: Span,
    },
    /// Query expression: `from ... [where/let/join/order/limit/group] select ...`.
    Query {
        clauses: Vec<QueryClause>,
        span: Span,
    },
    /// Table constructor: `table [ {..}, {..} ]` (optionally `table key(..) [..]`).
    TableConstructor { rows: Vec<Expr>, span: Span },
    /// `start <function-call>` action expression.
    Start { call: Box<Expr>, span: Span },
    /// Object-constructor expression: `[service|client] object { members }`.
    ObjectConstructor { members: Vec<Stmt>, span: Span },
}

/// A clause within a query expression.
#[derive(Debug)]
#[allow(dead_code)]
pub enum QueryClause {
    /// `from <binding> in <source>` (binding may destructure into several names).
    From { vars: Vec<String>, source: Expr },
    /// `where <condition>`.
    Where(Expr),
    /// `let <bindings>`.
    Let(Vec<LetBinding>),
    /// `[outer] join <binding> in <source> on <lhs> equals <rhs>`.
    Join {
        vars: Vec<String>,
        source: Box<Expr>,
        on_left: Box<Expr>,
        on_right: Box<Expr>,
    },
    /// `order by <key> [ascending|descending], ...`.
    OrderBy(Vec<Expr>),
    /// `limit <expr>`.
    Limit(Expr),
    /// `select <expr>`.
    Select(Expr),
    /// `do { ... }` query action clause.
    Do(Vec<Stmt>),
    /// `group by ...` / `on conflict ...` — retained without detail.
    Other,
}

/// A single binding within a `let` expression.
#[derive(Debug)]
#[allow(dead_code)]
pub struct LetBinding {
    pub name: String,
    pub value: Expr,
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
            | Expr::FieldAccess { span, .. }
            | Expr::MemberAssign { span, .. }
            | Expr::MethodCall { span, .. }
            | Expr::ArrayLiteral { span, .. }
            | Expr::MapLiteral { span, .. }
            | Expr::Ternary { span, .. }
            | Expr::Elvis { span, .. }
            | Expr::Range { span, .. }
            | Expr::Cast { span, .. }
            | Expr::New { span, .. }
            | Expr::NamedArg { span, .. }
            | Expr::Check { span, .. }
            | Expr::TypeOf { span, .. }
            | Expr::StringTemplate { span, .. }
            | Expr::TypeTest { span, .. }
            | Expr::Let { span, .. }
            | Expr::AnonFunction { span, .. }
            | Expr::Arrow { span, .. }
            | Expr::RemoteCall { span, .. }
            | Expr::Query { span, .. }
            | Expr::TableConstructor { span, .. }
            | Expr::Start { span, .. }
            | Expr::ObjectConstructor { span, .. } => span,
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
    /// A foreach loop statement. `variable` is the primary binding; a
    /// destructuring binding (`foreach [T,U] [a, b] in ...`) additionally
    /// populates `extra_bindings` with the remaining bound names.
    Foreach {
        type_annotation: Option<TypeDescriptor>,
        variable: String,
        extra_bindings: Vec<String>,
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
    /// A module-level type definition: `type Name <descriptor>;`.
    TypeDef {
        is_public: bool,
        name: String,
        name_span: Span,
        descriptor: TypeDescriptor,
        span: Span,
    },
    /// An enum definition: `enum Name { A, B = expr, ... }`.
    EnumDef {
        is_public: bool,
        name: String,
        name_span: Span,
        members: Vec<EnumMember>,
        span: Span,
    },
    /// A class definition: `class Name { fields and methods }`.
    ClassDef {
        is_public: bool,
        name: String,
        name_span: Span,
        qualifiers: Vec<String>,
        members: Vec<Stmt>,
        span: Span,
    },
    /// A service declaration. The header is parsed leniently; the body is parsed
    /// into member statements (fields and resource/remote methods).
    ServiceDecl { members: Vec<Stmt>, span: Span },
    /// A listener declaration: `listener Type name = expr;`.
    ListenerDecl {
        name: String,
        name_span: Span,
        span: Span,
    },
    /// An annotation declaration. Parsed leniently.
    AnnotationDecl { span: Span },
    /// An `xmlns` namespace declaration. Parsed leniently.
    Xmlns { span: Span },
    /// A `match` statement.
    Match {
        subject: Expr,
        arms: Vec<MatchArm>,
        span: Span,
    },
    /// A bare block statement `{ ... }`.
    Block { body: Vec<Stmt>, span: Span },
    /// A destructuring variable declaration binding several names from a list or
    /// mapping pattern, e.g. `var [a, b] = t;` or `Rec {x, y} = r;`.
    DestructureDecl {
        names: Vec<String>,
        name_spans: Vec<Span>,
        initializer: Expr,
        span: Span,
    },
    /// A `fail <expr>;` statement.
    Fail { value: Expr, span: Span },
    /// A `lock { ... }` statement.
    Lock { body: Vec<Stmt>, span: Span },
    /// A `do { ... } [on fail [error] <var> { ... }]` statement.
    DoOnFail {
        body: Vec<Stmt>,
        on_fail_var: Option<String>,
        on_fail_body: Vec<Stmt>,
        span: Span,
    },
    /// A `transaction { ... }` statement.
    Transaction { body: Vec<Stmt>, span: Span },
    /// A `retry[ transaction] { ... }` statement.
    Retry { body: Vec<Stmt>, span: Span },
    /// A `rollback [<expr>];` statement.
    Rollback { span: Span },
    /// A `fork { ... }` statement. Body is parsed leniently.
    Fork { span: Span },
    /// A named `worker <name> [returns T] { ... }` declaration.
    Worker {
        name: String,
        body: Vec<Stmt>,
        span: Span,
    },
}

/// A single arm of a `match` statement.
#[derive(Debug)]
#[allow(dead_code)]
pub struct MatchArm {
    /// One or more alternative patterns (`p1 | p2 | ...`).
    pub patterns: Vec<MatchPattern>,
    /// Names bound by the patterns, made available in the arm body.
    pub bindings: Vec<String>,
    /// Optional `if <guard>` condition.
    pub guard: Option<Expr>,
    pub body: Vec<Stmt>,
}

/// A structural pattern used in a `match` arm.
#[derive(Debug)]
#[allow(dead_code)]
pub enum MatchPattern {
    /// `_` wildcard.
    Wildcard,
    /// A literal value pattern (rendered text form retained).
    Literal(String),
    /// A capture/const-reference pattern binding an identifier.
    Binding(String),
    /// A list pattern `[p, ...]`.
    List(Vec<MatchPattern>),
    /// A mapping pattern `{ key: p, ... }`.
    Mapping(Vec<(String, MatchPattern)>),
    /// An error pattern `error(p, ...)`.
    Error(Vec<MatchPattern>),
    /// A rest pattern `...var`.
    Rest(String),
}

/// A single member of an enum definition.
#[derive(Debug)]
#[allow(dead_code)]
pub struct EnumMember {
    pub name: String,
    pub name_span: Span,
    pub value: Option<Expr>,
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
            | Stmt::Function { span, .. }
            | Stmt::TypeDef { span, .. }
            | Stmt::EnumDef { span, .. }
            | Stmt::ClassDef { span, .. }
            | Stmt::ServiceDecl { span, .. }
            | Stmt::ListenerDecl { span, .. }
            | Stmt::AnnotationDecl { span, .. }
            | Stmt::Xmlns { span, .. }
            | Stmt::Match { span, .. }
            | Stmt::Block { span, .. }
            | Stmt::DestructureDecl { span, .. }
            | Stmt::Fail { span, .. }
            | Stmt::Lock { span, .. }
            | Stmt::DoOnFail { span, .. }
            | Stmt::Transaction { span, .. }
            | Stmt::Retry { span, .. }
            | Stmt::Rollback { span, .. }
            | Stmt::Fork { span, .. }
            | Stmt::Worker { span, .. } => span,
        }
    }
}
