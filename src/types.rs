use std::{collections::{HashMap, HashSet}, mem::discriminant};
use crate::ast::{BinaryOp, Expr, Statement, Type, TypeKind, UnaryOp};

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    pub fn new(message: impl Into<String>) -> Self {
        TypeError { message: message.into() }
    }
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    structs: HashMap<String, Vec<(String, Type)>>,
    errors: HashSet<String>,
    current_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
            errors: HashSet::new(),
            current_return_type: None,
        }
    }


    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }


    fn types_equal(&self, a: &Type, b: &Type) -> bool {
        return a == b;
    }

    fn is_assignable(&self, from: &Type, to: &Type) -> bool {
        if from.kind == TypeKind::Null {
            return to.nullable;
        }
        
        if from.kind == TypeKind::Unknown {
            return (from.nullable == to.nullable || to.nullable)
                && (from.errorable == to.errorable || to.errorable);
        }
        
        from.kind == to.kind 
            && (from.nullable == to.nullable || to.nullable) 
            && (from.errorable == to.errorable || to.errorable)
    }

    fn is_numeric(&self, ty: &Type) -> bool {
        if ty.kind != TypeKind::Primitive("integer".to_string()) 
        && ty.kind != TypeKind::Primitive("float".to_string()) {
            return false;
        }
        true
    }

    fn is_boolean(&self, ty: &Type) -> bool {
        if ty.kind != TypeKind::Primitive("boolean".to_string()) {
            return false;
        }
        true
    }


    pub fn check_expr(&mut self, expr: &Expr) -> Result<Type, TypeError> {
        match expr {
            Expr::Null => Ok(Type {
                kind: TypeKind::Null,
                nullable: true,
                errorable: false,
            }),

            Expr::Integer(_) => {
                Ok(Type {
                    kind: TypeKind::Primitive("integer".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            Expr::Float(_) => {
                Ok(Type {
                    kind: TypeKind::Primitive("float".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            Expr::String(_) => {
                Ok(Type {
                    kind: TypeKind::Primitive("string".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            Expr::Boolean(_) => {
                Ok(Type {
                    kind: TypeKind::Primitive("boolean".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            Expr::Identifier(name) => {
                match self.lookup(name) {
                    Some(ty) => Ok(ty.clone()),
                    None => Err(TypeError::new(format!("Undefined identifier '{}'", name))),
                }
            }   
            Expr::List(elements) => {
                if elements.is_empty() {
                    return Ok(Type {
                        kind: TypeKind::List(Box::new(Type {
                            kind: TypeKind::Unknown,
                            nullable: false,
                            errorable: false,
                        })),
                        nullable: false,
                        errorable: false,
                    });
                } else {
                    let mut first_type = self.check_expr(&elements[0])?;
                    for elem in elements.iter().skip(1) {
                        let elem_type = self.check_expr(elem)?;
                        if self.is_assignable(&first_type, &elem_type) {
                            first_type = elem_type;
                        } else if self.is_assignable(&elem_type, &first_type) {
                            continue;
                        } else {
                            return Err(TypeError::new("Incompatible types in list literal"));
                        }
                    }
                    Ok(Type {
                        kind: TypeKind::List(Box::new(first_type)),
                        nullable: false,
                        errorable: false,
                    })
                }
            }
            Expr::Dict(pairs) => {
                if pairs.is_empty() {
                    return Ok(Type {
                        kind: TypeKind::Dict { 
                            key_type: Box::new(Type {
                                kind: TypeKind::Unknown,
                                nullable: false,
                                errorable: false,
                            }),
                            value_type: Box::new(Type {
                                kind: TypeKind::Unknown,
                                nullable: false,
                                errorable: false,
                            }),
                        },
                        nullable: false,
                        errorable: false,
                    });
                } else {
                    let mut first_key_type = self.check_expr(&pairs[0].0)?;
                    let mut first_value_type = self.check_expr(&pairs[0].1)?;

                    for (key, value) in pairs.iter().skip(1) {
                        let key_type = self.check_expr(key)?;
                        let value_type = self.check_expr(value)?;

                        if self.is_assignable(&first_key_type, &key_type) {
                            first_key_type = key_type;
                        } else if self.is_assignable(&key_type, &first_key_type) {
                            continue;
                        } else {
                            return Err(TypeError::new("Incompatible key types in dict literal"));
                        }

                        if self.is_assignable(&first_value_type, &value_type) {
                            first_value_type = value_type;
                        } else if self.is_assignable(&value_type, &first_value_type) {
                            continue;
                        } else {
                            return Err(TypeError::new("Incompatible value types in dict literal"));
                        }
                    }

                    Ok(Type {
                        kind: TypeKind::Dict { 
                            key_type: Box::new(first_key_type),
                            value_type: Box::new(first_value_type),
                        },
                        nullable: false,
                        errorable: false,
                    })
                }
            }
            Expr::MemberAccess { object, field } => {
                let struct_type = self.check_expr(object)?;

                if let TypeKind::Primitive(a) = struct_type.kind && !struct_type.nullable && !struct_type.errorable {
                    self.structs.get(&a)
                        .and_then(|fields| fields.iter().find(|(fname, _)| fname == field))
                        .map(|(_, ftype)| ftype.clone())
                        .ok_or_else(|| TypeError::new(format!("Type '{}' has no field '{}'", a, field)))
                } else {
                    Err(TypeError::new("Member access on non-struct type"))
                }
            }
            Expr::KeyAccess { dict, key } => {
                let dict_key_type = self.check_expr(key)?;
                let dict_type = self.check_expr(dict)?;

                if let TypeKind::Dict { key_type, value_type } = &dict_type.kind && !dict_type.nullable && !dict_type.errorable {
                    if self.is_assignable(&dict_key_type, key_type) {
                        Ok(value_type.as_ref().clone())
                    } else {
                        Err(TypeError::new("Incompatible key type for dict access"))
                    }
                } else if let TypeKind::List(inner_type) = &dict_type.kind && !dict_type.nullable && !dict_type.errorable {
                    if dict_key_type.kind == TypeKind::Primitive("integer".to_string())
                        && !dict_key_type.nullable
                        && !dict_key_type.errorable {
                            Ok(inner_type.as_ref().clone())
                    } else {
                        Err(TypeError::new("List index must be of type integer"))
                    }
                } else {
                    Err(TypeError::new("Key access on non-dict and non-list type"))
                }
            }
            Expr::Init { name, fields } => {
                let struct_fields = self.structs.get(name)
                    .ok_or_else(|| TypeError::new(format!("Undefined struct '{}'", name)))?
                    .clone();

                if struct_fields.len() != fields.len() {
                    return Err(TypeError::new(format!(
                        "Struct '{}' expects {} fields, got {}",
                        name,
                        struct_fields.len(),
                        fields.len()
                    )));
                }

                for (field_name, field_expr) in fields {
                    let expected_field = struct_fields.iter().find(|(fname, _)| fname == field_name);
                    match expected_field {
                        Some((_, expected_type)) => {
                            let expr_type = self.check_expr(field_expr)?;
                            if !self.is_assignable(&expr_type, expected_type) {
                                return Err(TypeError::new(format!(
                                    "Incompatible type for field '{}' in struct '{}'",
                                    field_name, name
                                )));
                            }
                        }
                        None => {
                            return Err(TypeError::new(format!(
                                "Struct '{}' has no field '{}'",
                                name, field_name
                            )));
                        }
                    }
                }

                Ok(Type {
                    kind: TypeKind::Primitive(name.clone()),
                    nullable: false,
                    errorable: false,
                })
            }
            Expr::Binary { left, op, right } => {
                self.check_binary(left, op, right)
            }
            Expr::Unary { op, expr } => {
                self.check_unary(op, expr)
            }
            Expr::Call { callee, args } => {
                let callee_type = self.check_expr(callee)?;

                if let TypeKind::Function { param_types, return_type } = callee_type.kind && !callee_type.nullable && !callee_type.errorable {
                    if param_types.len() != args.len() {
                        return Err(TypeError::new("Incorrect number of arguments in function call"));
                    }

                    for (i, arg) in args.iter().enumerate() {
                        let arg_type = self.check_expr(arg)?;
                        if !self.is_assignable(&arg_type, &param_types[i]) {
                            return Err(TypeError::new("Incompatible argument type in function call"));
                        }
                    }

                    Ok(*return_type)
                } else {
                    Err(TypeError::new("Callee is not a function"))
                }
            }
            Expr::NotNull(expr) => {
                if self.check_expr(expr)?.nullable {
                    let inner_type = self.check_expr(expr)?;
                    Ok(Type {
                        kind: inner_type.kind,
                        nullable: false,
                        errorable: inner_type.errorable,
                    })
                } else {
                    Err(TypeError::new("Expression is not nullable"))
                }
            }
            Expr::NotError(expr) => {
                if self.check_expr(expr)?.errorable {
                    let inner_type = self.check_expr(expr)?;
                    Ok(Type {
                        kind: inner_type.kind,
                        nullable: inner_type.nullable,
                        errorable: false,
                    })
                } else {
                    Err(TypeError::new("Expression is not errorable"))
                }
            }
            Expr::NotNullOrError(expr) => {
                let inner_type = self.check_expr(expr)?;
                if inner_type.nullable && inner_type.errorable {
                    Ok(Type {
                        kind: inner_type.kind,
                        nullable: false,
                        errorable: false,
                    })
                } else {
                    Err(TypeError::new("Expression is neither nullable nor errorable"))
                }
            }
        }
    }

    fn check_binary(&mut self, left: &Expr, op: &BinaryOp, right: &Expr) -> Result<Type, TypeError> {
        let left_ty = self.check_expr(left)?;
        let right_ty = self.check_expr(right)?;

        match op {
            BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Power => {
                if !self.is_numeric(&left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new("Left operand must be a non-nullable, non-errorable numeric type"));
                }
                if !self.is_numeric(&right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new("Right operand must be a non-nullable, non-errorable numeric type"));
                }
                // If either is float, result is float
                let is_float = left_ty.kind == TypeKind::Primitive("float".to_string())
                    || right_ty.kind == TypeKind::Primitive("float".to_string());
                Ok(Type {
                    kind: TypeKind::Primitive(if is_float { "float" } else { "integer" }.to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            BinaryOp::And | BinaryOp::Or => {
                if !self.is_boolean(&left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new("Left operand must be a non-nullable, non-errorable boolean"));
                }
                if !self.is_boolean(&right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new("Right operand must be a non-nullable, non-errorable boolean"));
                }
                Ok(Type {
                    kind: TypeKind::Primitive("boolean".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            BinaryOp::Eq | BinaryOp::Neq => {
                if left_ty.nullable || left_ty.errorable || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new("Cannot compare nullable or errorable types"));
                }
                if left_ty.kind != right_ty.kind {
                    return Err(TypeError::new("Cannot compare values of different types"));
                }
                Ok(Type {
                    kind: TypeKind::Primitive("boolean".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Lte | BinaryOp::Gte => {
                if !self.is_numeric(&left_ty) || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new("Left operand must be a non-nullable, non-errorable numeric type"));
                }
                if !self.is_numeric(&right_ty) || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new("Right operand must be a non-nullable, non-errorable numeric type"));
                }
                Ok(Type {
                    kind: TypeKind::Primitive("boolean".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            BinaryOp::BitwiseAnd | BinaryOp::BitwiseOr | BinaryOp::Xor | BinaryOp::Sll | BinaryOp::Srl => {
                let int_kind = TypeKind::Primitive("integer".to_string());
                if left_ty.kind != int_kind || left_ty.nullable || left_ty.errorable {
                    return Err(TypeError::new("Left operand must be a non-nullable, non-errorable integer"));
                }
                if right_ty.kind != int_kind || right_ty.nullable || right_ty.errorable {
                    return Err(TypeError::new("Right operand must be a non-nullable, non-errorable integer"));
                }
                Ok(Type {
                    kind: int_kind,
                    nullable: false,
                    errorable: false,
                })
            }
            BinaryOp::Is => {
                // Assignment: check that right is assignable to left
                if !self.is_assignable(&right_ty, &left_ty) {
                    return Err(TypeError::new("Cannot assign: incompatible types"));
                }
                Ok(left_ty)
            }
        }
    }

    fn check_unary(&mut self, op: &UnaryOp, expr: &Expr) -> Result<Type, TypeError> {
        let expr_ty = self.check_expr(expr)?;

        match op {
            UnaryOp::Not => {
                if !self.is_boolean(&expr_ty) || expr_ty.nullable || expr_ty.errorable {
                    return Err(TypeError::new("Operand must be a non-nullable, non-errorable boolean"));
                }
                Ok(Type {
                    kind: TypeKind::Primitive("boolean".to_string()),
                    nullable: false,
                    errorable: false,
                })
            }
            UnaryOp::Minus => {
                if !self.is_numeric(&expr_ty) || expr_ty.nullable || expr_ty.errorable {
                    return Err(TypeError::new("Operand must be a non-nullable, non-errorable numeric type"));
                }
                Ok(expr_ty)
            }
            UnaryOp::Raise => {
                // Raise makes the expression errorable
                Ok(Type {
                    kind: expr_ty.kind,
                    nullable: expr_ty.nullable,
                    errorable: true,
                })
            }
        }
    }

    pub fn check_stmt(&mut self, stmt: &Statement) -> Result<(), TypeError> {
        match stmt {
            Statement::Expr(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
            Statement::Let { name, value, type_annotation } => {
                todo!()
            }
            Statement::Const { name, value, type_annotation } => {
                todo!()
            }
            Statement::Return(expr) => {
                todo!()
            }
            Statement::Break => {
                Ok(())
            }
            Statement::Continue => {
                Ok(())
            }
            Statement::If { condition, consequent, alternate } => {
                todo!()
            }
            Statement::For { initializer, condition, increment, body } => {
                todo!()
            }
            Statement::While { condition, body } => {
                todo!()
            }
            Statement::Function { name, params, return_type, body } => {
                todo!()
            }
            Statement::Struct { name, fields } => {
                todo!()
            }
            Statement::Error { name } => {
                todo!()
            }
            Statement::Match { expr, arms } => {
                todo!()
            }
        }
    }


    pub fn check_program(&mut self, statements: &[Statement]) -> Result<(), TypeError> {
        for stmt in statements {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }
}
