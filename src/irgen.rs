use crate::aast::{AnalyzedExpr, AnalyzedStatement, Expr};
use crate::ast::{Pattern, Type};
use crate::fast::FlattenedProgram;
use crate::ir::{IRExpr, IRFunction, IRPattern, IRProgram, IRStmt, IRStruct};

pub struct IRGenerator {
    structs: Vec<IRStruct>,
}

impl IRGenerator {
    pub fn new() -> Self {
        IRGenerator { structs: vec![] }
    }

    pub fn generate(&mut self, program: &FlattenedProgram) -> IRProgram {
        for stmt in &program.structs {
            let ir_struct = self.lower_struct(stmt);
            self.structs.push(ir_struct);
        }

        let mut functions = vec![];
        for stmt in &program.functions {
            let ir_func = self.lower_function(stmt);
            functions.push(ir_func);
        }

        IRProgram {
            structs: self.structs.clone(),
            functions,
        }
    }

    fn lower_struct(&mut self, stmt: &AnalyzedStatement) -> IRStruct {
        match stmt {
            AnalyzedStatement::Struct { name, fields } => {
                let mut offsets = vec![];
                let mut offset = 0u32;
                for (_, ty) in fields {
                    offsets.push(offset);
                    offset += 8;
                }
                IRStruct {
                    name: name.clone(),
                    fields: fields.clone(),
                    size: offset,
                    offsets,
                    kind: crate::ir::IRStructKind::Captures,
                }
            }
            _ => panic!("expected struct"),
        }
    }

    fn lower_function(&mut self, stmt: &AnalyzedStatement) -> IRFunction {
        match stmt {
            AnalyzedStatement::Function { name, params, returns, body, captured, index, fn_index, locals } => {
                let ir_body: Vec<IRStmt> = body.iter().map(|s| self.lower_stmt(s)).collect();
                IRFunction {
                    name: name.clone(),
                    params: params.iter().map(|(_, ty, _, _)| ty.clone()).collect(),
                    returns: returns.clone(),
                    locals: locals.clone(),
                    captures_struct: Some(self.lookup_struct(name)),
                    body: ir_body,
                    func_index: fn_index.unwrap(),
                }
            }
            _ => panic!("expected function"),
        }
    }

    fn lower_stmt(&mut self, stmt: &AnalyzedStatement) -> IRStmt {
        match stmt {
            AnalyzedStatement::Expr(expr) => {
                let ir_expr = self.lower_expr(expr);
                IRStmt::Expr(ir_expr)
            },
            AnalyzedStatement::Let { name, ty, value, captured, index } => {
                let ir_value = value.as_ref().map(|v| self.lower_expr(v));
                IRStmt::LocalSet {
                    value: ir_value.unwrap_or(IRExpr {
                        node: crate::ir::IRExprKind::Null,
                        ty: ty.clone(),
                    }),
                    index: index.unwrap(),
                }
            },
            AnalyzedStatement::Const { name, ty, value, captured, index } => {
                let ir_value = self.lower_expr(value);
                IRStmt::LocalSet {
                    value: ir_value,
                    index: index.unwrap(),
                }
            },
            AnalyzedStatement::Return(expr) => {
                let ir_expr = expr.as_ref().map(|e| self.lower_expr(e));
                IRStmt::Return(ir_expr)
            },
            AnalyzedStatement::Break => {
                IRStmt::Break
            },
            AnalyzedStatement::Continue => {
                IRStmt::Continue
            },
            AnalyzedStatement::If { condition, then_block, else_block } => {
                let ir_condition = self.lower_expr(condition);
                let ir_then_block: Vec<IRStmt> = then_block.iter().map(|s| self.lower_stmt(s)).collect();
                let ir_else_block: Option<Vec<IRStmt>> = else_block.as_ref().map(|stmts| {
                    stmts.iter().map(|s| self.lower_stmt(s)).collect()
                });
                IRStmt::If {
                    condition: ir_condition,
                    then_block: ir_then_block,
                    else_block: ir_else_block,
                }
            },
            AnalyzedStatement::For { init, condition, update, body } => {
                let ir_init = Box::new(self.lower_stmt(init));
                let ir_condition = self.lower_expr(condition);
                let ir_update = Box::new(self.lower_stmt(update));
                let ir_body: Vec<IRStmt> = body.iter().map(|s| self.lower_stmt(s)).collect();
                IRStmt::For {
                    init: ir_init,
                    condition: ir_condition,
                    update: ir_update,
                    body: ir_body,
                }
            },
            AnalyzedStatement::While { condition, body } => {
                let ir_condition = self.lower_expr(condition);
                let ir_body: Vec<IRStmt> = body.iter().map(|s| self.lower_stmt(s)).collect();
                IRStmt::While {
                    condition: ir_condition,
                    body: ir_body,
                }
            },
            AnalyzedStatement::Print(expr) => {
                let ir_expr = self.lower_expr(expr);
                IRStmt::Print(ir_expr)
            },
            AnalyzedStatement::Produce(expr) => {
                let ir_expr = self.lower_expr(expr);
                IRStmt::Produce(ir_expr)
            },
            AnalyzedStatement::Function { .. } => panic!("unexpected nested function after flattening"),
            AnalyzedStatement::Struct { .. } => panic!("unexpected struct in function body"),
            AnalyzedStatement::Error { .. } => panic!("unexpected error in function body"),
        }
    }

    fn lower_expr(&mut self, expr: &AnalyzedExpr) -> IRExpr {
        match &expr.expr {
            Expr::Null => {
                IRExpr {
                    node: crate::ir::IRExprKind::Null,
                    ty: expr.ty.clone(),
                }
            },
            Expr::Integer(val) => {
                IRExpr {
                    node: crate::ir::IRExprKind::Integer(*val),
                    ty: expr.ty.clone(),
                }
            },
            Expr::Float(val) => {
                IRExpr {
                    node: crate::ir::IRExprKind::Float(*val),
                    ty: expr.ty.clone(),
                }
            },
            Expr::String(val) => {
                IRExpr {
                    node: crate::ir::IRExprKind::String(val.clone()),
                    ty: expr.ty.clone(),
                }
            },
            Expr::Boolean(val) => {
                IRExpr {
                    node: crate::ir::IRExprKind::Boolean(*val),
                    ty: expr.ty.clone(),
                }
            },
            Expr::Identifier { name, index } => {
                IRExpr {
                    node: crate::ir::IRExprKind::Local(index.unwrap()),
                    ty: expr.ty.clone(),
                }
            },
            Expr::List(elements) => {
                IRExpr { node: crate::ir::IRExprKind::List(elements.iter().map(|e| self.lower_expr(e)).collect()), ty: expr.ty.clone() }
            },
            Expr::Field { object, field } => {
                let ir_object = self.lower_expr(object);
                let struct_name = match &object.ty.kind {
                    crate::ast::TypeKind::Struct { name } => name,
                    _ => panic!("expected struct type for field access"),
                };
                let offset = self.get_field_offset(struct_name, field);
                IRExpr {
                    node: crate::ir::IRExprKind::Field {
                        object: Box::new(ir_object),
                        offset,
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::Index { object, key } => {
                let ir_object = self.lower_expr(object);
                let ir_key = self.lower_expr(key);
                IRExpr {
                    node: crate::ir::IRExprKind::Index {
                        list: Box::new(ir_object),
                        index: Box::new(ir_key),
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::New { name, fields } => {
                let struct_index = self.lookup_struct(name);
                let ir_fields: Vec<IRExpr> = fields.iter().map(|(_, expr)| self.lower_expr(expr)).collect();
                IRExpr {
                    node: crate::ir::IRExprKind::New {
                        struct_index,
                        fields: ir_fields,
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::Binary { left, op, right } => {
                let ir_left = self.lower_expr(left);
                let ir_right = self.lower_expr(right);
                IRExpr {
                    node: crate::ir::IRExprKind::Binary {
                        left: Box::new(ir_left),
                        op: op.clone(),
                        right: Box::new(ir_right),
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::Unary { op, expr: inner } => {
                let ir_inner = self.lower_expr(inner);
                IRExpr {
                    node: crate::ir::IRExprKind::Unary {
                        op: op.clone(),
                        expr: Box::new(ir_inner),
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::Call { callee, args } => {
                let ir_callee = self.lower_expr(callee);
                let ir_args: Vec<IRExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                IRExpr {
                    node: crate::ir::IRExprKind::Call {
                        callee: Box::new(ir_callee),
                        args: ir_args,
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::Match { .. } => todo!(),
            Expr::Closure { fn_index, captures } => {
                let ir_captures = self.lower_expr(captures);
                IRExpr {
                    node: crate::ir::IRExprKind::Closure {
                        fn_index: *fn_index,
                        captures: vec![ir_captures],
                    },
                    ty: expr.ty.clone(),
                }
            },
            Expr::UnwrapError(inner) => {
                let ir_inner = self.lower_expr(inner);
                IRExpr {
                    node: crate::ir::IRExprKind::UnwrapError(Box::new(ir_inner)),
                    ty: expr.ty.clone(),
                }
            },
            Expr::UnwrapNull(inner) => {
                let ir_inner = self.lower_expr(inner);
                IRExpr {
                    node: crate::ir::IRExprKind::UnwrapNull(Box::new(ir_inner)),
                    ty: expr.ty.clone(),
                }
            },
        }
    }

    fn lower_pattern(&mut self, pattern: &Pattern) -> IRPattern {
        match pattern {
            Pattern::MatchNull => todo!(),
            Pattern::MatchError => todo!(),
            Pattern::MatchType(ty) => todo!(),
            Pattern::MatchAll => todo!(),
        }
    }

    fn lookup_struct(&self, name: &str) -> u32 {
        self.structs.iter().position(|s| s.name == name).expect("struct not found") as u32
    }

    fn get_field_offset(&self, struct_name: &str, field_name: &str) -> u32 {
        let structure = self.structs.iter().find(|s| s.name == struct_name).expect("struct not found");
        let mut offset: u32 = 0;
        for (name, ty) in &structure.fields {
            if name == field_name {
                return offset;
            }
            offset += 8; // assuming each field is 8 bytes for simplicity
        }
        panic!("field not found");
    }
}
