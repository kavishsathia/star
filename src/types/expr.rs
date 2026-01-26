use super::{TypeChecker, TypeError};
use crate::ast::{self, Type, TypeKind};
use crate::tast::{self, TypedExpr};

impl TypeChecker {
    pub fn check_expr(&mut self, expr: &ast::Expr) -> Result<TypedExpr, TypeError> {
        match expr {
            ast::Expr::Null => Ok(TypedExpr {
                expr: tast::Expr::Null,
                ty: Type {
                    kind: TypeKind::Null,
                    nullable: true,
                    errorable: false,
                },
            }),

            ast::Expr::Integer(n) => Ok(TypedExpr {
                expr: tast::Expr::Integer(*n),
                ty: Type {
                    kind: TypeKind::Integer,
                    nullable: false,
                    errorable: false,
                },
            }),

            ast::Expr::Float(n) => Ok(TypedExpr {
                expr: tast::Expr::Float(*n),
                ty: Type {
                    kind: TypeKind::Float,
                    nullable: false,
                    errorable: false,
                },
            }),

            ast::Expr::String(s) => Ok(TypedExpr {
                expr: tast::Expr::String(s.clone()),
                ty: Type {
                    kind: TypeKind::String,
                    nullable: false,
                    errorable: false,
                },
            }),

            ast::Expr::Boolean(b) => Ok(TypedExpr {
                expr: tast::Expr::Boolean(*b),
                ty: Type {
                    kind: TypeKind::Boolean,
                    nullable: false,
                    errorable: false,
                },
            }),

            ast::Expr::Identifier(name) => match self.lookup(name) {
                Some(ty) => Ok(TypedExpr {
                    expr: tast::Expr::Identifier(name.clone()),
                    ty: ty.clone(),
                }),
                None => Err(TypeError::new(format!("Undefined identifier '{}'", name))),
            },

            ast::Expr::List(elements) => {
                if elements.is_empty() {
                    let ty = Type {
                        kind: TypeKind::List {
                            element: Box::new(Type {
                                kind: TypeKind::Unknown,
                                nullable: false,
                                errorable: false,
                            }),
                        },
                        nullable: false,
                        errorable: false,
                    };
                    Ok(TypedExpr {
                        expr: tast::Expr::List(vec![]),
                        ty,
                    })
                } else {
                    let mut typed_elements = Vec::new();
                    let first_typed = self.check_expr(&elements[0])?;
                    let mut element_type = first_typed.ty.clone();
                    typed_elements.push(first_typed);

                    for elem in elements.iter().skip(1) {
                        let typed_elem = self.check_expr(elem)?;
                        if self.is_assignable(&element_type, &typed_elem.ty) {
                            element_type = typed_elem.ty.clone();
                        } else if !self.is_assignable(&typed_elem.ty, &element_type) {
                            return Err(TypeError::new("Incompatible types in list literal"));
                        }
                        typed_elements.push(typed_elem);
                    }

                    Ok(TypedExpr {
                        expr: tast::Expr::List(typed_elements),
                        ty: Type {
                            kind: TypeKind::List {
                                element: Box::new(element_type),
                            },
                            nullable: false,
                            errorable: false,
                        },
                    })
                }
            }

            ast::Expr::Field { object, field } => {
                let typed_object = self.check_expr(object)?;

                if let TypeKind::Struct { name } = &typed_object.ty.kind {
                    if typed_object.ty.nullable || typed_object.ty.errorable {
                        return Err(TypeError::new("Field access on nullable or errorable type"));
                    }
                    let field_type = self
                        .structs
                        .get(name)
                        .and_then(|fields| fields.0.iter().find(|(fname, _)| fname == field))
                        .map(|(_, ftype)| ftype.clone())
                        .ok_or_else(|| {
                            TypeError::new(format!("Type '{}' has no field '{}'", name, field))
                        })?;

                    Ok(TypedExpr {
                        expr: tast::Expr::Field {
                            object: Box::new(typed_object),
                            field: field.clone(),
                        },
                        ty: field_type,
                    })
                } else {
                    Err(TypeError::new("Field access on non-struct type"))
                }
            }

            ast::Expr::Index { object, key } => {
                let typed_key = self.check_expr(key)?;
                let typed_object = self.check_expr(object)?;

                if let TypeKind::List { element } = &typed_object.ty.kind {
                    if typed_object.ty.nullable || typed_object.ty.errorable {
                        return Err(TypeError::new("Index access on nullable or errorable type"));
                    }
                    if typed_key.ty.kind == TypeKind::Integer
                        && !typed_key.ty.nullable
                        && !typed_key.ty.errorable
                    {
                        let elem_type = element.as_ref().clone();
                        Ok(TypedExpr {
                            expr: tast::Expr::Index {
                                object: Box::new(typed_object),
                                key: Box::new(typed_key),
                            },
                            ty: elem_type,
                        })
                    } else {
                        Err(TypeError::new("List index must be of type integer"))
                    }
                } else {
                    Err(TypeError::new("Index access on non-list type"))
                }
            }

            ast::Expr::Slice { expr, start, end } => {
                let typed_expr = self.check_expr(expr)?;
                let typed_start = self.check_expr(start)?;
                let typed_end = self.check_expr(end)?;

                if let TypeKind::List { element } = &typed_expr.ty.kind {
                    if typed_expr.ty.nullable || typed_expr.ty.errorable {
                        return Err(TypeError::new("Slice access on nullable or errorable type"));
                    }
                    if typed_start.ty.kind == TypeKind::Integer
                        && !typed_start.ty.nullable
                        && !typed_start.ty.errorable
                        && typed_end.ty.kind == TypeKind::Integer
                        && !typed_end.ty.nullable
                        && !typed_end.ty.errorable
                    {
                        let elem_type = element.as_ref().clone();
                        Ok(TypedExpr {
                            expr: tast::Expr::Slice {
                                expr: Box::new(typed_expr),
                                start: Box::new(typed_start),
                                end: Box::new(typed_end),
                            },
                            ty: Type {
                                kind: TypeKind::List {
                                    element: Box::new(elem_type),
                                },
                                nullable: false,
                                errorable: false,
                            },
                        })
                    } else {
                        Err(TypeError::new("Slice indices must be of type integer"))
                    }
                } else {
                    Err(TypeError::new("Slice access on non-list type"))
                }
            }

            ast::Expr::New { name, fields } => {
                let struct_fields = self
                    .structs
                    .get(name)
                    .ok_or_else(|| TypeError::new(format!("Undefined struct '{}'", name)))?
                    .clone();

                if struct_fields.0.len() != fields.len() {
                    return Err(TypeError::new(format!(
                        "Struct '{}' expects {} fields, got {}",
                        name,
                        struct_fields.0.len(),
                        fields.len()
                    )));
                }

                let mut typed_fields = Vec::new();
                for (field_name, field_expr) in fields {
                    let expected_field = struct_fields
                        .0
                        .iter()
                        .find(|(fname, _)| fname == field_name);
                    match expected_field {
                        Some((_, expected_type)) => {
                            let typed_expr = self.check_expr(field_expr)?;
                            if !self.is_assignable(&typed_expr.ty, expected_type) {
                                return Err(TypeError::new(format!(
                                    "Incompatible type for field '{}' in struct '{}'",
                                    field_name, name
                                )));
                            }
                            typed_fields.push((field_name.clone(), typed_expr));
                        }
                        None => {
                            return Err(TypeError::new(format!(
                                "Struct '{}' has no field '{}'",
                                name, field_name
                            )));
                        }
                    }
                }

                Ok(TypedExpr {
                    expr: tast::Expr::New {
                        name: name.clone(),
                        fields: typed_fields,
                    },
                    ty: Type {
                        kind: TypeKind::Struct { name: name.clone() },
                        nullable: false,
                        errorable: false,
                    },
                })
            }

            ast::Expr::Binary { left, op, right } => {
                let typed_left = self.check_expr(left)?;
                let typed_right = self.check_expr(right)?;
                let result_ty = self.check_binary_types(&typed_left.ty, op, &typed_right.ty)?;

                Ok(TypedExpr {
                    expr: tast::Expr::Binary {
                        left: Box::new(typed_left),
                        op: op.clone(),
                        right: Box::new(typed_right),
                    },
                    ty: result_ty,
                })
            }

            ast::Expr::Unary { op, expr } => {
                let typed_expr = self.check_expr(expr)?;
                let result_ty = self.check_unary_types(op, &typed_expr.ty)?;

                Ok(TypedExpr {
                    expr: tast::Expr::Unary {
                        op: op.clone(),
                        expr: Box::new(typed_expr),
                    },
                    ty: result_ty,
                })
            }

            ast::Expr::Call { callee, args } => {
                let typed_callee = self.check_expr(callee)?;

                if let TypeKind::Function { params, returns } = &typed_callee.ty.kind {
                    if typed_callee.ty.nullable || typed_callee.ty.errorable {
                        return Err(TypeError::new("Cannot call nullable or errorable function"));
                    }
                    if params.len() != args.len() {
                        return Err(TypeError::new(
                            "Incorrect number of arguments in function call",
                        ));
                    }

                    let params = params.clone();
                    let return_ty = returns.as_ref().clone();

                    let mut typed_args = Vec::new();
                    for (i, arg) in args.iter().enumerate() {
                        let typed_arg = self.check_expr(arg)?;
                        if !self.is_assignable(&typed_arg.ty, &params[i]) {
                            return Err(TypeError::new(
                                "Incompatible argument type in function call",
                            ));
                        }
                        typed_args.push(typed_arg);
                    }

                    Ok(TypedExpr {
                        expr: tast::Expr::Call {
                            callee: Box::new(typed_callee),
                            args: typed_args,
                        },
                        ty: return_ty,
                    })
                } else {
                    Err(TypeError::new("Callee is not a function"))
                }
            }

            ast::Expr::Match {
                expr,
                binding,
                arms,
            } => {
                todo!()
            }

            ast::Expr::UnwrapNull(inner) => {
                let typed_inner = self.check_expr(inner)?;
                if typed_inner.ty.nullable {
                    let result_ty = Type {
                        kind: typed_inner.ty.kind.clone(),
                        nullable: false,
                        errorable: typed_inner.ty.errorable,
                    };
                    Ok(TypedExpr {
                        expr: tast::Expr::UnwrapNull(Box::new(typed_inner)),
                        ty: result_ty,
                    })
                } else {
                    Err(TypeError::new("Expression is not nullable"))
                }
            }

            ast::Expr::UnwrapError(inner) => {
                let typed_inner = self.check_expr(inner)?;
                if typed_inner.ty.errorable {
                    let result_ty = Type {
                        kind: typed_inner.ty.kind.clone(),
                        nullable: typed_inner.ty.nullable,
                        errorable: false,
                    };
                    Ok(TypedExpr {
                        expr: tast::Expr::UnwrapError(Box::new(typed_inner)),
                        ty: result_ty,
                    })
                } else {
                    Err(TypeError::new("Expression is not errorable"))
                }
            }
        }
    }

    fn check_binary_types(
        &self,
        left_ty: &Type,
        op: &ast::BinaryOp,
        right_ty: &Type,
    ) -> Result<Type, TypeError> {
        match op {
            ast::BinaryOp::Plus => {
                if left_ty.kind == TypeKind::String && right_ty.kind == TypeKind::String {
                    if left_ty.nullable
                        || left_ty.errorable
                        || right_ty.nullable
                        || right_ty.errorable
                    {
                        return Err(TypeError::new(
                            "String operands must be non-nullable and non-errorable",
                        ));
                    }
                    return Ok(Type {
                        kind: TypeKind::String,
                        nullable: false,
                        errorable: false,
                    });
                }
                if let (
                    TypeKind::List { element: left_elem },
                    TypeKind::List {
                        element: right_elem,
                    },
                ) = (&left_ty.kind, &right_ty.kind)
                {
                    if left_elem != right_elem {
                        return Err(TypeError::new(
                            "List element types must match for concatenation",
                        ));
                    }
                    if left_ty.nullable
                        || left_ty.errorable
                        || right_ty.nullable
                        || right_ty.errorable
                    {
                        return Err(TypeError::new(
                            "List operands must be non-nullable and non-errorable",
                        ));
                    }
                    return Ok(Type {
                        kind: TypeKind::List {
                            element: left_elem.clone(),
                        },
                        nullable: false,
                        errorable: false,
                    });
                }
                if !self.is_numeric(left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new(
                        "Left operand must be a non-nullable, non-errorable numeric type, string, or list",
                    ));
                }
                if !self.is_numeric(right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new(
                        "Right operand must be a non-nullable, non-errorable numeric type, string, or list",
                    ));
                }
                let is_float = left_ty.kind == TypeKind::Float || right_ty.kind == TypeKind::Float;
                Ok(Type {
                    kind: if is_float {
                        TypeKind::Float
                    } else {
                        TypeKind::Integer
                    },
                    nullable: false,
                    errorable: false,
                })
            }
            ast::BinaryOp::Minus
            | ast::BinaryOp::Multiply
            | ast::BinaryOp::Divide
            | ast::BinaryOp::Power
            | ast::BinaryOp::Modulo => {
                if !self.is_numeric(left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new(
                        "Left operand must be a non-nullable, non-errorable numeric type",
                    ));
                }
                if !self.is_numeric(right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new(
                        "Right operand must be a non-nullable, non-errorable numeric type",
                    ));
                }
                let is_float = left_ty.kind == TypeKind::Float || right_ty.kind == TypeKind::Float;
                Ok(Type {
                    kind: if is_float {
                        TypeKind::Float
                    } else {
                        TypeKind::Integer
                    },
                    nullable: false,
                    errorable: false,
                })
            }
            ast::BinaryOp::And | ast::BinaryOp::Or => {
                if !self.is_boolean(left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new(
                        "Left operand must be a non-nullable, non-errorable boolean",
                    ));
                }
                if !self.is_boolean(right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new(
                        "Right operand must be a non-nullable, non-errorable boolean",
                    ));
                }
                Ok(Type {
                    kind: TypeKind::Boolean,
                    nullable: false,
                    errorable: false,
                })
            }
            ast::BinaryOp::Eq | ast::BinaryOp::Neq => {
                if left_ty.nullable || left_ty.errorable || right_ty.nullable || right_ty.errorable
                {
                    return Err(TypeError::new("Cannot compare nullable or errorable types"));
                }
                if left_ty.kind != right_ty.kind {
                    return Err(TypeError::new("Cannot compare values of different types"));
                }
                Ok(Type {
                    kind: TypeKind::Boolean,
                    nullable: false,
                    errorable: false,
                })
            }
            ast::BinaryOp::Lt | ast::BinaryOp::Gt | ast::BinaryOp::Lte | ast::BinaryOp::Gte => {
                if !self.is_numeric(left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new(
                        "Left operand must be a non-nullable, non-errorable numeric type",
                    ));
                }
                if !self.is_numeric(right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new(
                        "Right operand must be a non-nullable, non-errorable numeric type",
                    ));
                }
                Ok(Type {
                    kind: TypeKind::Boolean,
                    nullable: false,
                    errorable: false,
                })
            }
            ast::BinaryOp::BitwiseAnd
            | ast::BinaryOp::BitwiseOr
            | ast::BinaryOp::Xor
            | ast::BinaryOp::Sll
            | ast::BinaryOp::Srl => {
                if left_ty.kind != TypeKind::Integer || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new(
                        "Left operand must be a non-nullable, non-errorable integer",
                    ));
                }
                if right_ty.kind != TypeKind::Integer || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new(
                        "Right operand must be a non-nullable, non-errorable integer",
                    ));
                }
                Ok(Type {
                    kind: TypeKind::Integer,
                    nullable: false,
                    errorable: false,
                })
            }
            ast::BinaryOp::Is => {
                if !self.is_assignable(right_ty, left_ty) {
                    return Err(TypeError::new("Cannot assign: incompatible types"));
                }
                Ok(left_ty.clone())
            }
            &ast::BinaryOp::In => {
                if let TypeKind::List { element } = &right_ty.kind {
                    if right_ty.nullable || right_ty.errorable {
                        return Err(TypeError::new(
                            "Right operand must be a non-nullable, non-errorable list",
                        ));
                    }
                    if !self.is_assignable(left_ty, element) {
                        return Err(TypeError::new(
                            "Left operand type is not compatible with list element type",
                        ));
                    }
                    Ok(Type {
                        kind: TypeKind::Boolean,
                        nullable: false,
                        errorable: false,
                    })
                } else {
                    Err(TypeError::new("Right operand must be a list"))
                }
            }
        }
    }

    fn check_unary_types(&self, op: &ast::UnaryOp, expr_ty: &Type) -> Result<Type, TypeError> {
        match op {
            ast::UnaryOp::Not => {
                if !self.is_boolean(expr_ty) || expr_ty.nullable || expr_ty.errorable {
                    return Err(TypeError::new(
                        "Operand must be a non-nullable, non-errorable boolean",
                    ));
                }
                Ok(Type {
                    kind: TypeKind::Boolean,
                    nullable: false,
                    errorable: false,
                })
            }
            ast::UnaryOp::Minus => {
                if !self.is_numeric(expr_ty) || expr_ty.nullable || expr_ty.errorable {
                    return Err(TypeError::new(
                        "Operand must be a non-nullable, non-errorable numeric type",
                    ));
                }
                Ok(expr_ty.clone())
            }
            ast::UnaryOp::Raise => {
                if let TypeKind::Struct { name } = &expr_ty.kind {
                    if !self.errors.contains(name) {
                        return Err(TypeError::new(&format!(
                            "'{}' is not an error type",
                            name
                        )));
                    }
                } else {
                    return Err(TypeError::new("Can only raise error types"));
                }
                Ok(Type {
                    kind: expr_ty.kind.clone(),
                    nullable: expr_ty.nullable,
                    errorable: true,
                })
            }
            &ast::UnaryOp::Count => {
                if let TypeKind::List { .. } = &expr_ty.kind {
                    if expr_ty.nullable || expr_ty.errorable {
                        return Err(TypeError::new(
                            "Operand must be a non-nullable, non-errorable list",
                        ));
                    }
                    Ok(Type {
                        kind: TypeKind::Integer,
                        nullable: false,
                        errorable: false,
                    })
                } else {
                    Err(TypeError::new("Operand must be a list"))
                }
            }
            &ast::UnaryOp::Stringify => {
                if expr_ty.nullable || expr_ty.errorable {
                    return Err(TypeError::new(
                        "Operand must be non-nullable and non-errorable",
                    ));
                }
                Ok(Type {
                    kind: TypeKind::String,
                    nullable: false,
                    errorable: false,
                })
            }
        }
    }
}
