#[derive(Debug)]
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
    Is
}

#[derive(Debug)]
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
    Null,     // only assignable to nullable types
    Unknown,  // assignable to any type (for empty collections)
    Primitive(String),
    List(Box<Type>),
    Dict { key_type: Box<Type>, value_type: Box<Type> },
    Function { param_types: Vec<Type>, return_type: Box<Type> },
}

#[derive(Debug)]
pub enum Expr {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Identifier(String),
    List(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>), 
    MemberAccess { object: Box<Expr>, field: String },
    KeyAccess { dict: Box<Expr>, key: Box<Expr> },
    Init { name: String, fields: Vec<(String, Expr)> },
    Binary { left: Box<Expr>, op: BinaryOp, right: Box<Expr> },
    Unary { op: UnaryOp, expr: Box<Expr> },
    Call { callee: Box<Expr>, args: Vec<Expr> },
    NotNull(Box<Expr>),
    NotError(Box<Expr>),
    NotNullOrError(Box<Expr>),
}

#[derive(Debug)]
pub enum Statement {
    Expr(Expr),
    Let { name: String, value: Option<Box<Expr>>, type_annotation: Type },
    Const { name: String, value: Box<Expr>, type_annotation: Type },
    Return(Option<Box<Expr>>),
    Break,
    Continue,
    If { condition: Box<Expr>, consequent: Vec<Statement>, alternate: Option<Vec<Statement>> },
    For { initializer: Box<Statement>, condition: Box<Expr>, increment: Box<Statement>, body: Vec<Statement> },
    While { condition: Box<Expr>, body: Vec<Statement> },
    Function { name: String, params: Vec<(String, Type)>, return_type: Type, body: Vec<Statement> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Error {name: String},
    Match { expr: Box<Expr>, arms: Vec<(String, Vec<Statement>)> },
}
