use crate::aast::{AnalyzedExpr, AnalyzedStatement, Expr};
use crate::ast::{BinaryOp, Pattern, Type};
use crate::error::CompilerError;
use crate::fast::FlattenedProgram;
use crate::ir::{IRExpr, IRFunction, IRPattern, IRProgram, IRStmt, IRStruct};

pub struct IRGenerator {
    structs: Vec<IRStruct>,
}

impl IRGenerator {
    pub fn new() -> Self {
        IRGenerator { structs: vec![] }
    }

    pub fn generate(&mut self, program: &FlattenedProgram) -> Result<IRProgram, CompilerError> {
        for stmt in &program.structs {
            let ir_struct = self.lower_struct(stmt)?;
            self.structs.push(ir_struct);
        }

        let mut functions = vec![];
        for stmt in &program.functions {
            let ir_func = self.lower_function(stmt)?;
            functions.push(ir_func);
        }

        Ok(IRProgram {
            structs: self.structs.clone(),
            functions,
        })
    }

    fn lower_struct(&mut self, entry: &(AnalyzedStatement, u32, u32)) -> Result<IRStruct, CompilerError> {
        let (stmt, struct_count, list_count) = entry;
        match stmt {
            AnalyzedStatement::Struct { name, fields } => {
                let mut offsets = vec![];
                let mut offset = 0u32;
                for _ in fields {
                    offsets.push(offset);
                    offset += 8;
                }
                Ok(IRStruct {
                    name: name.clone(),
                    fields: fields.clone(),
                    size: offset,
                    offsets,
                    struct_count: *struct_count,
                    list_count: *list_count,
                    kind: crate::ir::IRStructKind::Captures,
                })
            }
            _ => Err(CompilerError::IRGen {
                message: "expected struct".to_string(),
            }),
        }
    }

    fn lower_function(&mut self, stmt: &AnalyzedStatement) -> Result<IRFunction, CompilerError> {
        match stmt {
            AnalyzedStatement::Function {
                name,
                params,
                returns,
                body,
                captured,
                index,
                fn_index,
                locals,
            } => {
                let mut ir_body = Vec::new();
                for s in body {
                    ir_body.push(self.lower_stmt(s)?);
                }
                Ok(IRFunction {
                    name: name.clone(),
                    params: params.iter().map(|(_, ty, _, _)| ty.clone()).collect(),
                    returns: returns.clone(),
                    locals: locals.clone(),
                    captures_struct: Some(self.lookup_struct(name)?),
                    body: ir_body,
                    func_index: fn_index.unwrap(),
                })
            }
            _ => Err(CompilerError::IRGen {
                message: "expected function".to_string(),
            }),
        }
    }

    fn lower_stmt(&mut self, stmt: &AnalyzedStatement) -> Result<IRStmt, CompilerError> {
        match stmt {
            AnalyzedStatement::Expr(expr) => {
                let ir_expr = self.lower_expr(expr)?;
                Ok(IRStmt::Expr(ir_expr))
            }
            AnalyzedStatement::Let {
                name,
                ty,
                value,
                captured,
                index,
            } => {
                let ir_value = match value {
                    Some(v) => self.lower_expr(v)?,
                    None => IRExpr {
                        node: crate::ir::IRExprKind::Null,
                        ty: ty.clone(),
                    },
                };
                Ok(IRStmt::LocalSet {
                    value: ir_value,
                    index: index.unwrap(),
                })
            }
            AnalyzedStatement::Const {
                name,
                ty,
                value,
                captured,
                index,
            } => {
                let ir_value = self.lower_expr(value)?;
                Ok(IRStmt::LocalSet {
                    value: ir_value,
                    index: index.unwrap(),
                })
            }
            AnalyzedStatement::Return(expr) => {
                let ir_expr = match expr {
                    Some(e) => Some(self.lower_expr(e)?),
                    None => None,
                };
                Ok(IRStmt::Return(ir_expr))
            }
            AnalyzedStatement::Break => Ok(IRStmt::Break),
            AnalyzedStatement::Continue => Ok(IRStmt::Continue),
            AnalyzedStatement::If {
                condition,
                then_block,
                else_block,
            } => {
                let ir_condition = self.lower_expr(condition)?;
                let mut ir_then_block = Vec::new();
                for s in then_block {
                    ir_then_block.push(self.lower_stmt(s)?);
                }
                let ir_else_block = match else_block {
                    Some(stmts) => {
                        let mut result = Vec::new();
                        for s in stmts {
                            result.push(self.lower_stmt(s)?);
                        }
                        Some(result)
                    }
                    None => None,
                };
                Ok(IRStmt::If {
                    condition: ir_condition,
                    then_block: ir_then_block,
                    else_block: ir_else_block,
                })
            }
            AnalyzedStatement::For {
                init,
                condition,
                update,
                body,
            } => {
                let ir_init = Box::new(self.lower_stmt(init)?);
                let ir_condition = self.lower_expr(condition)?;
                let ir_update = Box::new(self.lower_stmt(update)?);
                let mut ir_body = Vec::new();
                for s in body {
                    ir_body.push(self.lower_stmt(s)?);
                }
                Ok(IRStmt::For {
                    init: ir_init,
                    condition: ir_condition,
                    update: ir_update,
                    body: ir_body,
                })
            }
            AnalyzedStatement::While { condition, body } => {
                let ir_condition = self.lower_expr(condition)?;
                let mut ir_body = Vec::new();
                for s in body {
                    ir_body.push(self.lower_stmt(s)?);
                }
                Ok(IRStmt::While {
                    condition: ir_condition,
                    body: ir_body,
                })
            }
            AnalyzedStatement::Print(expr) => {
                let ir_expr = self.lower_expr(expr)?;
                Ok(IRStmt::Print(ir_expr))
            }
            AnalyzedStatement::Produce(expr) => {
                let ir_expr = self.lower_expr(expr)?;
                Ok(IRStmt::Produce(ir_expr))
            }
            AnalyzedStatement::Function { .. } => {
                Err(CompilerError::IRGen {
                    message: "unexpected nested function after flattening".to_string(),
                })
            }
            AnalyzedStatement::Struct { .. } => Err(CompilerError::IRGen {
                message: "unexpected struct in function body".to_string(),
            }),
            AnalyzedStatement::Error { .. } => Err(CompilerError::IRGen {
                message: "unexpected error in function body".to_string(),
            }),
            AnalyzedStatement::LocalClosure {
                fn_index,
                captures,
                index,
            } => {
                let ir_captures = self.lower_expr(captures)?;
                Ok(IRStmt::LocalClosure {
                    captures: Box::new(ir_captures),
                    index: *index,
                    fn_index: *fn_index,
                })
            }
        }
    }

    fn lower_expr(&mut self, expr: &AnalyzedExpr) -> Result<IRExpr, CompilerError> {
        match &expr.expr {
            Expr::Null => Ok(IRExpr {
                node: crate::ir::IRExprKind::Null,
                ty: expr.ty.clone(),
            }),
            Expr::Integer(val) => Ok(IRExpr {
                node: crate::ir::IRExprKind::Integer(*val),
                ty: expr.ty.clone(),
            }),
            Expr::Float(val) => Ok(IRExpr {
                node: crate::ir::IRExprKind::Float(*val),
                ty: expr.ty.clone(),
            }),
            Expr::String(val) => Ok(IRExpr {
                node: crate::ir::IRExprKind::String(val.clone()),
                ty: expr.ty.clone(),
            }),
            Expr::Boolean(val) => Ok(IRExpr {
                node: crate::ir::IRExprKind::Boolean(*val),
                ty: expr.ty.clone(),
            }),
            Expr::Identifier { name, index } => Ok(IRExpr {
                node: crate::ir::IRExprKind::Local(index.unwrap()),
                ty: expr.ty.clone(),
            }),
            Expr::List(elements) => {
                let mut ir_elements = Vec::new();
                for e in elements {
                    ir_elements.push(self.lower_expr(e)?);
                }
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::List(ir_elements),
                    ty: expr.ty.clone(),
                })
            }
            Expr::Field { object, field } => {
                let ir_object = self.lower_expr(object)?;
                let struct_name = match &object.ty.kind {
                    crate::ast::TypeKind::Struct { name } => name,
                    _ => return Err(CompilerError::IRGen {
                        message: "expected struct type for field access".to_string(),
                    }),
                };
                let offset = self.get_field_offset(struct_name, field)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::Field {
                        object: Box::new(ir_object),
                        offset,
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::Index { object, key } => {
                let ir_object = self.lower_expr(object)?;
                let ir_key = self.lower_expr(key)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::Index {
                        list: Box::new(ir_object),
                        index: Box::new(ir_key),
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::Slice { expr, start, end } => {
                let ir_expr = self.lower_expr(expr)?;
                let ir_start = self.lower_expr(start)?;
                let ir_end = self.lower_expr(end)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::Slice {
                        expr: Box::new(ir_expr),
                        start: Box::new(ir_start),
                        end: Box::new(ir_end),
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::New { name, fields } => {
                let struct_index = self.lookup_struct(name)?;
                let mut ir_fields = Vec::new();
                for (_, e) in fields {
                    ir_fields.push(self.lower_expr(e)?);
                }
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::New {
                        struct_index,
                        fields: ir_fields,
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::Binary {
                left,
                op: BinaryOp::Is,
                right,
            } => match &left.expr {
                Expr::Identifier { name: _, index } => {
                    let ir_left = self.lower_expr(left)?;
                    let ir_right = self.lower_expr(right)?;
                    Ok(IRExpr {
                        node: crate::ir::IRExprKind::Binary {
                            left: Box::new(ir_left),
                            op: BinaryOp::Is,
                            right: Box::new(ir_right),
                        },
                        ty: expr.ty.clone(),
                    })
                }
                Expr::Field { object, field } => {
                    let ir_object = self.lower_expr(&object)?;
                    let struct_name = match &object.ty.kind {
                        crate::ast::TypeKind::Struct { name } => name,
                        _ => return Err(CompilerError::IRGen {
                            message: "expected struct type for field access".to_string(),
                        }),
                    };
                    let offset = self.get_field_offset(struct_name, &field)?;
                    let ir_left = IRExpr {
                        node: crate::ir::IRExprKind::FieldReference {
                            object: Box::new(ir_object),
                            offset,
                        },
                        ty: left.ty.clone(),
                    };
                    let ir_right = self.lower_expr(right)?;
                    Ok(IRExpr {
                        node: crate::ir::IRExprKind::Binary {
                            left: Box::new(ir_left),
                            op: BinaryOp::Is,
                            right: Box::new(ir_right),
                        },
                        ty: expr.ty.clone(),
                    })
                }
                Expr::Index { object, key } => {
                    let ir_object = self.lower_expr(object)?;
                    let ir_key = self.lower_expr(key)?;
                    let ir_left = IRExpr {
                        node: crate::ir::IRExprKind::IndexReference {
                            list: Box::new(ir_object),
                            index: Box::new(ir_key),
                        },
                        ty: left.ty.clone(),
                    };
                    let ir_right = self.lower_expr(right)?;
                    Ok(IRExpr {
                        node: crate::ir::IRExprKind::Binary {
                            left: Box::new(ir_left),
                            op: BinaryOp::Is,
                            right: Box::new(ir_right),
                        },
                        ty: expr.ty.clone(),
                    })
                }
                _ => Err(CompilerError::IRGen {
                    message: "Left side of 'is' must be a local or field".to_string(),
                }),
            },
            Expr::Binary { left, op, right } => {
                let ir_left = self.lower_expr(left)?;
                let ir_right = self.lower_expr(right)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::Binary {
                        left: Box::new(ir_left),
                        op: op.clone(),
                        right: Box::new(ir_right),
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::Unary { op, expr: inner } => {
                let ir_inner = self.lower_expr(inner)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::Unary {
                        op: op.clone(),
                        expr: Box::new(ir_inner),
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::Call { callee, args } => {
                let ir_callee = self.lower_expr(callee)?;
                let mut ir_args = Vec::new();
                for a in args {
                    ir_args.push(self.lower_expr(a)?);
                }
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::Call {
                        callee: Box::new(ir_callee),
                        args: ir_args,
                    },
                    ty: expr.ty.clone(),
                })
            }
            Expr::Match { .. } => todo!(),
            Expr::UnwrapError(inner) => {
                let ir_inner = self.lower_expr(inner)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::UnwrapError(Box::new(ir_inner)),
                    ty: expr.ty.clone(),
                })
            }
            Expr::UnwrapNull(inner) => {
                let ir_inner = self.lower_expr(inner)?;
                Ok(IRExpr {
                    node: crate::ir::IRExprKind::UnwrapNull(Box::new(ir_inner)),
                    ty: expr.ty.clone(),
                })
            }
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

    fn lookup_struct(&self, name: &str) -> Result<u32, CompilerError> {
        self.structs
            .iter()
            .position(|s| s.name == name)
            .map(|i| i as u32)
            .ok_or_else(|| CompilerError::IRGen {
                message: format!("struct '{}' not found", name),
            })
    }

    fn get_field_offset(&self, struct_name: &str, field_name: &str) -> Result<u32, CompilerError> {
        let structure = self
            .structs
            .iter()
            .find(|s| s.name == struct_name)
            .ok_or_else(|| CompilerError::IRGen {
                message: format!("struct '{}' not found", struct_name),
            })?;
        let mut offset: u32 = 0;
        for (name, _ty) in &structure.fields {
            if name == field_name {
                return Ok(offset);
            }
            offset += 8; // assuming each field is 8 bytes for simplicity
        }
        Err(CompilerError::IRGen {
            message: format!("field '{}' not found in struct '{}'", field_name, struct_name),
        })
    }
}
