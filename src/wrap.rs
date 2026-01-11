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
        for stmt in &program.structs {
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

    fn wrap_to_type(&self, expr: AnalyzedExpr, expected: &Type, is_raised: bool) -> AnalyzedExpr {
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

    fn wrap_expr(&self, expr: AnalyzedExpr) -> AnalyzedExpr {
        match expr.expr {
            Expr::Binary {
                left,
                op: BinaryOp::Is,
                right,
            } => {
                let wrapped_left = self.wrap_expr(*left);
                let wrapped_right =
                    self.wrap_to_type(self.wrap_expr(*right), &wrapped_left.ty, false);
                AnalyzedExpr {
                    ty: expr.ty.clone(),
                    expr: Expr::Binary {
                        left: Box::new(wrapped_left),
                        op: BinaryOp::Is,
                        right: Box::new(wrapped_right),
                    },
                }
            }
            Expr::Unary {
                expr,
                op: UnaryOp::Raise,
            } => {
                let wrapped_expr = self.wrap_expr(*expr);
                self.wrap_to_type(
                    wrapped_expr,
                    &self.current_return_type.as_ref().unwrap(),
                    true,
                )
            }
            Expr::New { name, fields } => {
                let wrapped_fields: Vec<(String, AnalyzedExpr)> = fields
                    .into_iter()
                    .map(|(field_name, field_expr)| {
                        let expected_type = self.get_field_type(&name, &field_name).unwrap();
                        (
                            field_name,
                            self.wrap_to_type(self.wrap_expr(field_expr), &expected_type, false),
                        )
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
                    let wrapped_args: Vec<AnalyzedExpr> = args
                        .into_iter()
                        .enumerate()
                        .map(|(i, arg_expr)| {
                            self.wrap_to_type(self.wrap_expr(arg_expr), &params[i], false)
                        })
                        .collect();

                    AnalyzedExpr {
                        ty: expr.ty.clone(),
                        expr: Expr::Call {
                            callee,
                            args: wrapped_args,
                        },
                    }
                }
                _ => panic!("Callee is not a function type"),
            },
            _ => expr,
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
                    Some(self.wrap_to_type(self.wrap_expr(v), &ty, false))
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
                let wrapped_value = self.wrap_to_type(self.wrap_expr(value), &ty, false);
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
                    Some(self.wrap_to_type(
                        self.wrap_expr(ret_expr),
                        self.current_return_type.as_ref().unwrap(),
                        false,
                    ))
                } else {
                    Some(self.wrap_to_type(
                        self.wrap_expr(AnalyzedExpr {
                            expr: Expr::Null,
                            ty: Type {
                                kind: TypeKind::Null,
                                nullable: false,
                                errorable: false,
                            },
                        }),
                        self.current_return_type.as_ref().unwrap(),
                        false,
                    ))
                };
                AnalyzedStatement::Return(wrapped_expr)
            }
            _ => stmt,
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

        let tagged_union_struct = AnalyzedStatement::Struct {
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
        };

        let mut structs = vec![tagged_union_struct];
        structs.extend(program.structs);

        FlattenedProgram {
            structs,
            functions: wrapped_functions,
        }
    }
}
