use super::ast::{BinaryOp, Type, UnaryOp};

#[derive(Debug)]
pub struct IRProgram {
    pub structs: Vec<IRStruct>,
    pub functions: Vec<IRFunction>,
}

#[derive(Debug, Clone)]
pub struct IRExpr {
    pub node: IRExprKind,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum IRExprKind {
    Integer(i64),   // ty
    Float(f64),     // ty
    Boolean(bool),  // ty
    String(String), // ty
    Null,           // ty

    Local(u32), // ty

    Binary {
        left: Box<IRExpr>,
        op: BinaryOp,
        right: Box<IRExpr>,
    }, // ty
    Unary {
        op: UnaryOp,
        expr: Box<IRExpr>,
    }, // ty

    Call {
        callee: Box<IRExpr>,
        args: Vec<IRExpr>,
    },

    List(Vec<IRExpr>),

    New {
        struct_index: u32,
        fields: Vec<IRExpr>,
    },
    Field {
        object: Box<IRExpr>,
        offset: u32,
    },
    FieldReference {
        object: Box<IRExpr>,
        offset: u32,
    },
    Index {
        list: Box<IRExpr>,
        index: Box<IRExpr>,
    },
    IndexReference {
        list: Box<IRExpr>,
        index: Box<IRExpr>,
    },
    Slice {
        expr: Box<IRExpr>,
        start: Box<IRExpr>,
        end: Box<IRExpr>,
    },

    Match {
        expr: Box<IRExpr>,
        binding: u32,
        arms: Vec<(IRPattern, Vec<IRStmt>)>,
    },

    UnwrapError(Box<IRExpr>),
    UnwrapNull(Box<IRExpr>),
}

#[derive(Debug, Clone)]
pub enum IRPattern {
    Null,
    Error,
    Type(u32),
    All,
}

#[derive(Debug, Clone)]
pub enum IRStmt {
    Expr(IRExpr),
    LocalSet {
        index: u32,
        value: IRExpr,
    },
    Return(Option<IRExpr>),
    Break,
    Continue,
    If {
        condition: IRExpr,
        then_block: Vec<IRStmt>,
        else_block: Option<Vec<IRStmt>>,
    },
    For {
        init: Box<IRStmt>,
        condition: IRExpr,
        update: Box<IRStmt>,
        body: Vec<IRStmt>,
    },
    While {
        condition: IRExpr,
        body: Vec<IRStmt>,
    },
    Print(IRExpr),
    Produce(IRExpr),
    Raise(IRExpr),
    LocalClosure {
        fn_index: u32,
        captures: Box<IRExpr>,
        index: u32,
    },
}

#[derive(Debug, Clone)]
pub struct IRFunction {
    pub name: String,
    pub params: Vec<Type>,
    pub returns: Type,
    pub locals: Vec<Type>,
    pub captures_struct: Option<u32>,
    pub body: Vec<IRStmt>,
    pub func_index: u32,
}

#[derive(Debug, Clone)]
pub struct IRStruct {
    pub name: String,
    pub fields: Vec<(String, Type)>,
    pub size: u32,
    pub offsets: Vec<u32>,
    pub kind: IRStructKind,
    pub struct_count: u32,
    pub list_count: u32,
}

#[derive(Debug, Clone)]
pub enum IRStructKind {
    User,
    Captures,
    Error,
}
