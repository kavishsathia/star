#[derive(Debug)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Plus,
    Minus,
    Multiply,
    Divide,
    And,
    Or,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    BitwiseAnd,
    BitwiseOr,
    Power,
    Sll,
    Srl,
    Xor,
    Is,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Not,
    Minus,
    Raise,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub kind: TypeKind,
    pub nullable: bool,
    pub errorable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeKind {
    Integer,
    Float,
    Boolean,
    String,
    Struct { name: String },
    Error { name: String },
    List { element: Box<Type> },
    Dict { key: Box<Type>, value: Box<Type> },
    Function { params: Vec<Type>, returns: Box<Type> },
    Null,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Identifier(String),
    List(Vec<Expr>),
    Field { object: Box<Expr>, field: String },
    Index { object: Box<Expr>, key: Box<Expr> },
    New { name: String, fields: Vec<(String, Expr)> },
    Binary { left: Box<Expr>, op:   BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    Match { expr: Box<Expr>, binding: String, arms: Vec<(Pattern, Vec<Statement>)> },
    UnwrapError(Box<Expr>),
    UnwrapNull(Box<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    MatchNull,
    MatchError,
    MatchAll,
    MatchType(Type)
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Expr(Expr),
    Let { name: String, ty: Type, value: Option<Expr> },
    Const { name: String, ty: Type, value: Expr },
    Return(Option<Expr>),
    Break,
    Continue,
    If { condition: Expr, then_block: Vec<Statement>, else_block: Option<Vec<Statement>> },
    For { init: Box<Statement>, condition: Expr, update: Box<Statement>, body: Vec<Statement> },
    While { condition: Expr, body: Vec<Statement> },
    Function { name: String, params: Vec<(String, Type)>, returns: Type, body: Vec<Statement> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Error { name: String },
    Print(Expr),
    Produce(Expr),
}
