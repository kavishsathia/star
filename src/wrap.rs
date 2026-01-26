use crate::aast::{AnalyzedExpr, AnalyzedStatement, Expr};
use crate::ast::{BinaryOp, Type, TypeKind, UnaryOp};
use crate::error::CompilerError;
use crate::fast::FlattenedProgram;
use std::collections::HashMap;

pub struct Wrapper {
    functions: HashMap<String, (Vec<Type>, Type)>, // name -> (param_types, return_type)
    structs: HashMap<String, Vec<(String, Type)>>, // name -> fields
    current_return_type: Option<Type>,
}

impl Wrapper {
    pub fn new() -> Self {
        Wrapper {
            functions: HashMap::new(),
            structs: HashMap::new(),
            current_return_type: None,
        }
    }

    fn build_lookups(&mut self, program: &FlattenedProgram) {
        for (stmt, _, _) in &program.structs {
            if let AnalyzedStatement::Struct { name, fields } = stmt {
                self.structs.insert(name.clone(), fields.clone());
            }
        }
        for stmt in &program.functions {
            if let AnalyzedStatement::Function {
                name,
                params,
                returns,
                ..
            } = stmt
            {
                let param_types: Vec<Type> =
                    params.iter().map(|(_, ty, _, _)| ty.clone()).collect();
                self.functions
                    .insert(name.clone(), (param_types, returns.clone()));
            }
        }
    }

    fn get_field_type(&self, struct_name: &str, field_name: &str) -> Option<Type> {
        self.structs.get(struct_name).and_then(|fields| {
            fields
                .iter()
                .find(|(name, _)| name == field_name)
                .map(|(_, ty)| ty.clone())
        })
    }

    fn wrap_to_type(
        &mut self,
        expr: AnalyzedExpr,
        expected: &Type,
        is_raised: bool,
    ) -> AnalyzedExpr {
        if expr.ty.kind == TypeKind::Null
            || is_raised
            || ((!expr.ty.errorable && !expr.ty.nullable)
                && (expected.errorable || expected.nullable))
        {
            return AnalyzedExpr {
                ty: expected.clone(),
                expr: Expr::New {
                    name: "".to_string(),
                    fields: vec![
                        (
                            "tag".to_string(),
                            AnalyzedExpr {
                                ty: Type {
                                    kind: TypeKind::Integer,
                                    nullable: false,
                                    errorable: false,
                                },
                                expr: Expr::Integer(if is_raised {
                                    1
                                } else if expr.ty.kind == TypeKind::Null {
                                    0
                                } else {
                                    2
                                }),
                            },
                        ),
                        ("value".to_string(), expr),
                    ],
                },
            };
        } else {
            return expr;
        }
    }

    fn wrap_expr(&mut self, expr: AnalyzedExpr) -> Result<AnalyzedExpr, CompilerError> {
        match expr.expr {
            Expr::Binary {
                left,
                op: BinaryOp::Is,
                right,
            } => {
                let wrapped_left = self.wrap_expr(*left)?;
                let wrapped_right_inner = self.wrap_expr(*right)?;
                let wrapped_right = self.wrap_to_type(wrapped_right_inner, &wrapped_left.ty, false);
                Ok(AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::Binary {
                        left: Box::new(wrapped_left),
                        op: BinaryOp::Is,
                        right: Box::new(wrapped_right),
                    },
                })
            }
            Expr::Binary { left, op, right } => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Binary {
                    left: Box::new(self.wrap_expr(*left)?),
                    op,
                    right: Box::new(self.wrap_expr(*right)?),
                },
            }),
            Expr::Unary { expr: inner, op } => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Unary {
                    expr: Box::new(self.wrap_expr(*inner)?),
                    op,
                },
            }),
            Expr::New { name, fields } => {
                let mut wrapped_fields = Vec::new();
                for (field_name, field_expr) in fields {
                    let wrapped = self.wrap_expr(field_expr)?;
                    if let Some(expected_type) = self.get_field_type(&name, &field_name) {
                        wrapped_fields.push((field_name, self.wrap_to_type(wrapped, &expected_type, false)));
                    } else {
                        wrapped_fields.push((field_name, wrapped));
                    }
                }

                Ok(AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::New {
                        name,
                        fields: wrapped_fields,
                    },
                })
            }
            Expr::Call { callee, args } => match &callee.ty.kind {
                TypeKind::Function { params, .. } => {
                    let params = params.clone();
                    let wrapped_callee = self.wrap_expr(*callee)?;
                    let mut wrapped_args = Vec::new();
                    for (i, arg_expr) in args.into_iter().enumerate() {
                        let wrapped = self.wrap_expr(arg_expr)?;
                        wrapped_args.push(self.wrap_to_type(wrapped, &params[i], false));
                    }

                    Ok(AnalyzedExpr {
                        ty: expr.ty.clone(),
                        expr: Expr::Call {
                            callee: Box::new(wrapped_callee),
                            args: wrapped_args,
                        },
                    })
                }
                _ => return Err(CompilerError::Codegen {
                    message: "Callee is not a function type".to_string(),
                }),
            },
            Expr::Field { object, field } => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Field {
                    object: Box::new(self.wrap_expr(*object)?),
                    field,
                },
            }),
            Expr::Index { object, key } => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Index {
                    object: Box::new(self.wrap_expr(*object)?),
                    key: Box::new(self.wrap_expr(*key)?),
                },
            }),
            Expr::Slice {
                expr: inner,
                start,
                end,
            } => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Slice {
                    expr: Box::new(self.wrap_expr(*inner)?),
                    start: Box::new(self.wrap_expr(*start)?),
                    end: Box::new(self.wrap_expr(*end)?),
                },
            }),
            Expr::List(elements) => {
                let mut wrapped = Vec::new();
                for e in elements {
                    wrapped.push(self.wrap_expr(e)?);
                }
                Ok(AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::List(wrapped),
                })
            }
            Expr::Match {
                expr: match_expr,
                binding,
                arms,
            } => {
                let mut wrapped_arms = Vec::new();
                for (pattern, stmts) in arms {
                    let mut wrapped_stmts = Vec::new();
                    for s in stmts {
                        wrapped_stmts.push(self.wrap_stmt(s)?);
                    }
                    wrapped_arms.push((pattern, wrapped_stmts));
                }
                Ok(AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::Match {
                        expr: Box::new(self.wrap_expr(*match_expr)?),
                        binding,
                        arms: wrapped_arms,
                    },
                })
            }
            Expr::UnwrapNull(inner) => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::UnwrapNull(Box::new(self.wrap_expr(*inner)?)),
            }),
            Expr::UnwrapError(inner) => Ok(AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::UnwrapError(Box::new(self.wrap_expr(*inner)?)),
            }),
            Expr::Null
            | Expr::Integer(_)
            | Expr::Float(_)
            | Expr::String(_)
            | Expr::Boolean(_)
            | Expr::Identifier { .. } => Ok(expr),
        }
    }

    fn wrap_stmt(&mut self, stmt: AnalyzedStatement) -> Result<AnalyzedStatement, CompilerError> {
        match stmt {
            AnalyzedStatement::Let {
                name,
                ty,
                value,
                captured,
                index,
            } => {
                let wrapped_value = if let Some(v) = value {
                    let wrapped = self.wrap_expr(v)?;
                    Some(self.wrap_to_type(wrapped, &ty, false))
                } else {
                    None
                };
                Ok(AnalyzedStatement::Let {
                    name,
                    ty,
                    value: wrapped_value,
                    captured,
                    index,
                })
            }
            AnalyzedStatement::Const {
                name,
                ty,
                value,
                captured,
                index,
            } => {
                let wrapped = self.wrap_expr(value)?;
                let wrapped_value = self.wrap_to_type(wrapped, &ty, false);
                Ok(AnalyzedStatement::Const {
                    name,
                    ty,
                    value: wrapped_value,
                    captured,
                    index,
                })
            }
            AnalyzedStatement::Return(expr) => {
                let wrapped_expr = if let Some(ret_expr) = expr {
                    let wrapped = self.wrap_expr(ret_expr)?;
                    let ret_type = self.current_return_type.as_ref().unwrap().clone();
                    Some(self.wrap_to_type(wrapped, &ret_type, false))
                } else {
                    let null_expr = AnalyzedExpr {
                        expr: Expr::Null,
                        ty: Type {
                            kind: TypeKind::Null,
                            nullable: false,
                            errorable: false,
                        },
                    };
                    let wrapped = self.wrap_expr(null_expr)?;
                    let ret_type = self.current_return_type.as_ref().unwrap().clone();
                    Some(self.wrap_to_type(wrapped, &ret_type, false))
                };
                Ok(AnalyzedStatement::Return(wrapped_expr))
            }
            AnalyzedStatement::Expr(e) => Ok(AnalyzedStatement::Expr(self.wrap_expr(e)?)),
            AnalyzedStatement::If {
                condition,
                then_block,
                else_block,
            } => {
                let mut wrapped_then = Vec::new();
                for s in then_block {
                    wrapped_then.push(self.wrap_stmt(s)?);
                }
                let wrapped_else = match else_block {
                    Some(stmts) => {
                        let mut result = Vec::new();
                        for s in stmts {
                            result.push(self.wrap_stmt(s)?);
                        }
                        Some(result)
                    }
                    None => None,
                };
                Ok(AnalyzedStatement::If {
                    condition: self.wrap_expr(condition)?,
                    then_block: wrapped_then,
                    else_block: wrapped_else,
                })
            }
            AnalyzedStatement::While { condition, body } => {
                let mut wrapped_body = Vec::new();
                for s in body {
                    wrapped_body.push(self.wrap_stmt(s)?);
                }
                Ok(AnalyzedStatement::While {
                    condition: self.wrap_expr(condition)?,
                    body: wrapped_body,
                })
            }
            AnalyzedStatement::For {
                init,
                condition,
                update,
                body,
            } => {
                let mut wrapped_body = Vec::new();
                for s in body {
                    wrapped_body.push(self.wrap_stmt(s)?);
                }
                Ok(AnalyzedStatement::For {
                    init: Box::new(self.wrap_stmt(*init)?),
                    condition: self.wrap_expr(condition)?,
                    update: Box::new(self.wrap_stmt(*update)?),
                    body: wrapped_body,
                })
            }
            AnalyzedStatement::Print(e) => Ok(AnalyzedStatement::Print(self.wrap_expr(e)?)),
            AnalyzedStatement::Produce(e) => Ok(AnalyzedStatement::Produce(self.wrap_expr(e)?)),
            AnalyzedStatement::Raise(e) => {
                let wrapped_expr = self.wrap_expr(e)?;
                let ret_type = self.current_return_type.as_ref().unwrap().clone();
                Ok(AnalyzedStatement::Raise(self.wrap_to_type(wrapped_expr, &ret_type, true)))
            }
            AnalyzedStatement::Break
            | AnalyzedStatement::Continue
            | AnalyzedStatement::Struct { .. }
            | AnalyzedStatement::Error { .. }
            | AnalyzedStatement::Function { .. }
            | AnalyzedStatement::LocalClosure { .. } => Ok(stmt),
        }
    }

    fn wrap_function(&mut self, stmt: AnalyzedStatement) -> Result<AnalyzedStatement, CompilerError> {
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
                self.current_return_type = Some(returns.clone());
                let mut wrapped_body = Vec::new();
                for s in body {
                    wrapped_body.push(self.wrap_stmt(s)?);
                }
                self.current_return_type = None;
                Ok(AnalyzedStatement::Function {
                    name,
                    params,
                    returns,
                    body: wrapped_body,
                    captured,
                    index,
                    fn_index,
                    locals,
                })
            }
            _ => Ok(stmt),
        }
    }

    pub fn wrap_program(&mut self, program: FlattenedProgram) -> Result<FlattenedProgram, CompilerError> {
        self.build_lookups(&program);

        let mut wrapped_functions = Vec::new();
        for f in program.functions {
            wrapped_functions.push(self.wrap_function(f)?);
        }

        let tagged_union_struct = (
            AnalyzedStatement::Struct {
                name: "".to_string(),
                fields: vec![
                    (
                        "tag".to_string(),
                        Type {
                            kind: TypeKind::Integer,
                            nullable: false,
                            errorable: false,
                        },
                    ),
                    (
                        "value".to_string(),
                        Type {
                            kind: TypeKind::Integer,
                            nullable: false,
                            errorable: false,
                        },
                    ),
                ],
            },
            0u32,
            0u32,
        );

        let mut structs = vec![tagged_union_struct];
        structs.extend(program.structs);

        Ok(FlattenedProgram {
            structs,
            functions: wrapped_functions,
        })
    }
}
