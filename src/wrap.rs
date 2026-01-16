use crate::aast::{AnalyzedExpr, AnalyzedStatement, Expr};
use crate::ast::{BinaryOp, Type, TypeKind, UnaryOp};
use crate::fast::FlattenedProgram;
use core::panic;
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

    fn wrap_expr(&mut self, expr: AnalyzedExpr) -> AnalyzedExpr {
        match expr.expr {
            Expr::Binary {
                left,
                op: BinaryOp::Is,
                right,
            } => {
                let wrapped_left = self.wrap_expr(*left);
                let wrapped_right_inner = self.wrap_expr(*right);
                let wrapped_right = self.wrap_to_type(wrapped_right_inner, &wrapped_left.ty, false);
                AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::Binary {
                        left: Box::new(wrapped_left),
                        op: BinaryOp::Is,
                        right: Box::new(wrapped_right),
                    },
                }
            }
            Expr::Binary { left, op, right } => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Binary {
                    left: Box::new(self.wrap_expr(*left)),
                    op,
                    right: Box::new(self.wrap_expr(*right)),
                },
            },
            Expr::Unary {
                expr: inner,
                op: UnaryOp::Raise,
            } => {
                let wrapped_expr = self.wrap_expr(*inner);
                let ret_type = self.current_return_type.as_ref().unwrap().clone();
                self.wrap_to_type(wrapped_expr, &ret_type, true)
            }
            Expr::Unary { expr: inner, op } => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Unary {
                    expr: Box::new(self.wrap_expr(*inner)),
                    op,
                },
            },
            Expr::New { name, fields } => {
                let wrapped_fields: Vec<(String, AnalyzedExpr)> = fields
                    .into_iter()
                    .map(|(field_name, field_expr)| {
                        let wrapped = self.wrap_expr(field_expr);
                        if let Some(expected_type) = self.get_field_type(&name, &field_name) {
                            (
                                field_name,
                                self.wrap_to_type(wrapped, &expected_type, false),
                            )
                        } else {
                            (field_name, wrapped)
                        }
                    })
                    .collect();

                AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::New {
                        name,
                        fields: wrapped_fields,
                    },
                }
            }
            Expr::Call { callee, args } => match &callee.ty.kind {
                TypeKind::Function { params, .. } => {
                    let params = params.clone();
                    let wrapped_callee = self.wrap_expr(*callee);
                    let wrapped_args: Vec<AnalyzedExpr> = args
                        .into_iter()
                        .enumerate()
                        .map(|(i, arg_expr)| {
                            let wrapped = self.wrap_expr(arg_expr);
                            self.wrap_to_type(wrapped, &params[i], false)
                        })
                        .collect();

                    AnalyzedExpr {
                        ty: expr.ty.clone(),
                        expr: Expr::Call {
                            callee: Box::new(wrapped_callee),
                            args: wrapped_args,
                        },
                    }
                }
                _ => panic!("Callee is not a function type"),
            },
            Expr::Field { object, field } => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Field {
                    object: Box::new(self.wrap_expr(*object)),
                    field,
                },
            },
            Expr::Index { object, key } => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Index {
                    object: Box::new(self.wrap_expr(*object)),
                    key: Box::new(self.wrap_expr(*key)),
                },
            },
            Expr::Slice {
                expr: inner,
                start,
                end,
            } => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Slice {
                    expr: Box::new(self.wrap_expr(*inner)),
                    start: Box::new(self.wrap_expr(*start)),
                    end: Box::new(self.wrap_expr(*end)),
                },
            },
            Expr::List(elements) => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::List(elements.into_iter().map(|e| self.wrap_expr(e)).collect()),
            },
            Expr::Match {
                expr: match_expr,
                binding,
                arms,
            } => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::Match {
                    expr: Box::new(self.wrap_expr(*match_expr)),
                    binding,
                    arms: arms
                        .into_iter()
                        .map(|(pattern, stmts)| {
                            (
                                pattern,
                                stmts.into_iter().map(|s| self.wrap_stmt(s)).collect(),
                            )
                        })
                        .collect(),
                },
            },
            Expr::UnwrapNull(inner) => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::UnwrapNull(Box::new(self.wrap_expr(*inner))),
            },
            Expr::UnwrapError(inner) => AnalyzedExpr {
                ty: expr.ty.clone(),
                expr: Expr::UnwrapError(Box::new(self.wrap_expr(*inner))),
            },
            Expr::Null
            | Expr::Integer(_)
            | Expr::Float(_)
            | Expr::String(_)
            | Expr::Boolean(_)
            | Expr::Identifier { .. } => expr,
        }
    }

    fn wrap_stmt(&mut self, stmt: AnalyzedStatement) -> AnalyzedStatement {
        match stmt {
            AnalyzedStatement::Let {
                name,
                ty,
                value,
                captured,
                index,
            } => {
                let wrapped_value = if let Some(v) = value {
                    let wrapped = self.wrap_expr(v);
                    Some(self.wrap_to_type(wrapped, &ty, false))
                } else {
                    None
                };
                AnalyzedStatement::Let {
                    name,
                    ty,
                    value: wrapped_value,
                    captured,
                    index,
                }
            }
            AnalyzedStatement::Const {
                name,
                ty,
                value,
                captured,
                index,
            } => {
                let wrapped = self.wrap_expr(value);
                let wrapped_value = self.wrap_to_type(wrapped, &ty, false);
                AnalyzedStatement::Const {
                    name,
                    ty,
                    value: wrapped_value,
                    captured,
                    index,
                }
            }
            AnalyzedStatement::Return(expr) => {
                let wrapped_expr = if let Some(ret_expr) = expr {
                    let wrapped = self.wrap_expr(ret_expr);
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
                    let wrapped = self.wrap_expr(null_expr);
                    let ret_type = self.current_return_type.as_ref().unwrap().clone();
                    Some(self.wrap_to_type(wrapped, &ret_type, false))
                };
                AnalyzedStatement::Return(wrapped_expr)
            }
            AnalyzedStatement::Expr(e) => AnalyzedStatement::Expr(self.wrap_expr(e)),
            AnalyzedStatement::If {
                condition,
                then_block,
                else_block,
            } => AnalyzedStatement::If {
                condition: self.wrap_expr(condition),
                then_block: then_block.into_iter().map(|s| self.wrap_stmt(s)).collect(),
                else_block: else_block
                    .map(|stmts| stmts.into_iter().map(|s| self.wrap_stmt(s)).collect()),
            },
            AnalyzedStatement::While { condition, body } => AnalyzedStatement::While {
                condition: self.wrap_expr(condition),
                body: body.into_iter().map(|s| self.wrap_stmt(s)).collect(),
            },
            AnalyzedStatement::For {
                init,
                condition,
                update,
                body,
            } => AnalyzedStatement::For {
                init: Box::new(self.wrap_stmt(*init)),
                condition: self.wrap_expr(condition),
                update: Box::new(self.wrap_stmt(*update)),
                body: body.into_iter().map(|s| self.wrap_stmt(s)).collect(),
            },
            AnalyzedStatement::Print(e) => AnalyzedStatement::Print(self.wrap_expr(e)),
            AnalyzedStatement::Produce(e) => AnalyzedStatement::Produce(self.wrap_expr(e)),
            AnalyzedStatement::Break
            | AnalyzedStatement::Continue
            | AnalyzedStatement::Struct { .. }
            | AnalyzedStatement::Error { .. }
            | AnalyzedStatement::Function { .. }
            | AnalyzedStatement::LocalClosure { .. } => stmt,
        }
    }

    fn wrap_function(&mut self, stmt: AnalyzedStatement) -> AnalyzedStatement {
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
                let wrapped_body: Vec<_> = body.into_iter().map(|s| self.wrap_stmt(s)).collect();
                self.current_return_type = None;
                AnalyzedStatement::Function {
                    name,
                    params,
                    returns,
                    body: wrapped_body,
                    captured,
                    index,
                    fn_index,
                    locals,
                }
            }
            _ => stmt,
        }
    }

    pub fn wrap_program(&mut self, program: FlattenedProgram) -> FlattenedProgram {
        self.build_lookups(&program);

        let wrapped_functions: Vec<_> = program
            .functions
            .into_iter()
            .map(|f| self.wrap_function(f))
            .collect();

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

        FlattenedProgram {
            structs,
            functions: wrapped_functions,
        }
    }
}
