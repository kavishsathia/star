use crate::ast::{Type, BinaryOp, UnaryOp, Pattern};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
pub struct AnalyzedProgram {
    pub statements: Vec<AnalyzedStatement>,
}

#[derive(Debug, Clone)]
pub struct AnalyzedExpr {
    pub expr: Expr,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum Expr {
    Null,
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Identifier { name: String, index: Option<u32> },
    List(Vec<AnalyzedExpr>),
    Field { object: Box<AnalyzedExpr>, field: String },
    Index { object: Box<AnalyzedExpr>, key: Box<AnalyzedExpr> },
    New { name: String, fields: Vec<(String, AnalyzedExpr)> },
    Binary { left: Box<AnalyzedExpr>, op:   BinaryOp, right: Box<AnalyzedExpr> },
    Unary { op: UnaryOp, expr: Box<AnalyzedExpr> },
    Call { callee: Box<AnalyzedExpr>, args: Vec<AnalyzedExpr> },
    Match { expr: Box<AnalyzedExpr>, binding: String, arms: Vec<(Pattern, Vec<AnalyzedStatement>)> },
    Closure { fn_index: u32, captures: Box<AnalyzedExpr> },
    UnwrapError(Box<AnalyzedExpr>),
    UnwrapNull(Box<AnalyzedExpr>),
}

#[derive(Debug, Clone)]
pub enum AnalyzedStatement {
    Expr(AnalyzedExpr),
    Let { name: String, ty: Type, value: Option<AnalyzedExpr>, captured: Rc<RefCell<Option<String>>>, index: Option<u32> },
    Const { name: String, ty: Type, value: AnalyzedExpr, captured: Rc<RefCell<Option<String>>>, index: Option<u32> },
    Return(Option<AnalyzedExpr>),
    Break,
    Continue,
    If { condition: AnalyzedExpr, then_block: Vec<AnalyzedStatement>, else_block: Option<Vec<AnalyzedStatement>> },
    For { init: Box<AnalyzedStatement>, condition: AnalyzedExpr, update: Box<AnalyzedStatement>, body: Vec<AnalyzedStatement> },
    While { condition: AnalyzedExpr, body: Vec<AnalyzedStatement> },
    Function { name: String, params: Vec<(String, Type, u32, Rc<RefCell<Option<String>>>)>, returns: Type, body: Vec<AnalyzedStatement>, captured: Rc<RefCell<Option<String>>>, index: Option<u32>, fn_index: Option<u32>, locals: Vec<Type> },
    Struct { name: String, fields: Vec<(String, Type)> },
    Error { name: String },
    Print(AnalyzedExpr),
    Produce(AnalyzedExpr),
}
