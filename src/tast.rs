use crate::ast::{Type, BinaryOp, UnaryOp, Pattern};

#[derive(Debug)]
pub struct TypedProgram {
    pub statements: Vec<TypedStatement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpr {
    pub expr: Expr,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Identifier(String),
    List(Vec<TypedExpr>),
    Field { object: Box<TypedExpr>, field: String },
    Index { object: Box<TypedExpr>, key: Box<TypedExpr> },
    New { name: String, fields: Vec<(String, TypedExpr)> },
    Binary { left: Box<TypedExpr>, op:   BinaryOp, right: Box<TypedExpr> },
    Unary { op: UnaryOp, expr: Box<TypedExpr> },
    Call { callee: Box<TypedExpr>, args: Vec<TypedExpr> },
    Match { expr: Box<TypedExpr>, binding: String, arms: Vec<(Pattern, Vec<TypedStatement>)> },
    UnwrapError(Box<TypedExpr>),
    UnwrapNull(Box<TypedExpr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStatement {
    Expr(TypedExpr),
    Let { name: String, ty: Type, value: Option<TypedExpr> },
    Const { name: String, ty: Type, value: TypedExpr },
    Return(Option<TypedExpr>),
    Break,
    Continue,
    If { condition: TypedExpr, then_block: Vec<TypedStatement>, else_block: Option<Vec<TypedStatement>> },
    For { init: Box<TypedStatement>, condition: TypedExpr, update: Box<TypedStatement>, body: Vec<TypedStatement> },
    While { condition: TypedExpr, body: Vec<TypedStatement> },
    Function { name: String, params: Vec<(String, Type)>, returns: Type, body: Vec<TypedStatement> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Error { name: String },
    Print(TypedExpr),
    Produce(TypedExpr),
}
