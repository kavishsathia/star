use super::{TypeChecker, TypeError};
use crate::ast::{self, Type, TypeKind};
use crate::ast::tast::{self, TypedProgram, TypedStatement};

impl TypeChecker {
    pub fn check_stmt(&mut self, stmt: &ast::Statement) -> Result<TypedStatement, TypeError> {
        match stmt {
            ast::Statement::Expr(expr) => {
                let typed_expr = self.check_expr(expr)?;
                Ok(TypedStatement::Expr(typed_expr))
            }

            ast::Statement::Let { name, value, ty } => {
                let typed_value = if let Some(init_expr) = value {
                    let mut typed_init = self.check_expr(init_expr)?;

                    if let TypeKind::List { element } = &typed_init.ty.kind {
                        if element.kind == TypeKind::Unknown {
                            typed_init.ty = ty.clone();
                        }
                    }

                    if !self.is_assignable(&typed_init.ty, ty) {
                        return Err(TypeError::new(format!(
                            "Incompatible type in let binding for '{}'",
                            name
                        )));
                    }
                    Some(typed_init)
                } else {
                    if !ty.nullable {
                        return Err(TypeError::new(format!(
                            "Let binding for '{}' without initializer must be nullable",
                            name
                        )));
                    }
                    None
                };
                self.define(name.clone(), ty.clone());
                Ok(TypedStatement::Let {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: typed_value,
                })
            }

            ast::Statement::Const { name, value, ty } => {
                let mut typed_value = self.check_expr(value)?;

                if let TypeKind::List { element } = &typed_value.ty.kind {
                    if element.kind == TypeKind::Unknown {
                        typed_value.ty = ty.clone();
                    }
                }

                if !self.is_assignable(&typed_value.ty, ty) {
                    return Err(TypeError::new(format!(
                        "Incompatible type in const binding for '{}'",
                        name
                    )));
                }
                self.define(name.clone(), ty.clone());
                Ok(TypedStatement::Const {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: typed_value,
                })
            }

            ast::Statement::Return(expr) => {
                let typed_expr = if let Some(ret_expr) = expr {
                    let typed_ret = self.check_expr(ret_expr)?;
                    if let Some(expected_type) = &self.current_return_type {
                        if !self.is_assignable(&typed_ret.ty, expected_type) {
                            return Err(TypeError::new("Incompatible return type"));
                        }
                    } else {
                        return Err(TypeError::new("Return statement outside of function"));
                    }
                    Some(typed_ret)
                } else {
                    if let Some(expected_type) = &self.current_return_type {
                        if !expected_type.nullable {
                            return Err(TypeError::new(
                                "Incompatible return type: expected non-void",
                            ));
                        }
                    } else {
                        return Err(TypeError::new("Return statement outside of function"));
                    }
                    None
                };
                Ok(TypedStatement::Return(typed_expr))
            }

            ast::Statement::Break => Ok(TypedStatement::Break),

            ast::Statement::Continue => Ok(TypedStatement::Continue),

            ast::Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let typed_condition = self.check_expr(condition)?;
                if !self.is_boolean(&typed_condition.ty)
                    || typed_condition.ty.nullable
                    || typed_condition.ty.errorable
                {
                    return Err(TypeError::new(
                        "If condition must be a non-nullable, non-errorable boolean",
                    ));
                }

                self.push_scope();
                let typed_then: Vec<TypedStatement> = then_block
                    .iter()
                    .map(|s| self.check_stmt(s))
                    .collect::<Result<_, _>>()?;
                self.pop_scope();

                let typed_else = if let Some(alt_stmts) = else_block {
                    self.push_scope();
                    let typed: Vec<TypedStatement> = alt_stmts
                        .iter()
                        .map(|s| self.check_stmt(s))
                        .collect::<Result<_, _>>()?;
                    self.pop_scope();
                    Some(typed)
                } else {
                    None
                };

                Ok(TypedStatement::If {
                    condition: typed_condition,
                    then_block: typed_then,
                    else_block: typed_else,
                })
            }

            ast::Statement::For {
                init,
                condition,
                update,
                body,
            } => {
                self.push_scope();
                let typed_init = self.check_stmt(init)?;

                let typed_condition = self.check_expr(condition)?;
                if !self.is_boolean(&typed_condition.ty)
                    || typed_condition.ty.nullable
                    || typed_condition.ty.errorable
                {
                    return Err(TypeError::new(
                        "For loop condition must be a non-nullable, non-errorable boolean",
                    ));
                }

                let typed_body: Vec<TypedStatement> = body
                    .iter()
                    .map(|s| self.check_stmt(s))
                    .collect::<Result<_, _>>()?;

                let typed_update = self.check_stmt(update)?;

                self.pop_scope();

                Ok(TypedStatement::For {
                    init: Box::new(typed_init),
                    condition: typed_condition,
                    update: Box::new(typed_update),
                    body: typed_body,
                })
            }

            ast::Statement::While { condition, body } => {
                let typed_condition = self.check_expr(condition)?;
                if !self.is_boolean(&typed_condition.ty)
                    || typed_condition.ty.nullable
                    || typed_condition.ty.errorable
                {
                    return Err(TypeError::new(
                        "While loop condition must be a non-nullable, non-errorable boolean",
                    ));
                }

                self.push_scope();
                let typed_body: Vec<TypedStatement> = body
                    .iter()
                    .map(|s| self.check_stmt(s))
                    .collect::<Result<_, _>>()?;
                self.pop_scope();

                Ok(TypedStatement::While {
                    condition: typed_condition,
                    body: typed_body,
                })
            }

            ast::Statement::Function {
                name,
                params,
                returns,
                body,
            } => {
                let func_type = Type {
                    kind: TypeKind::Function {
                        params: params.iter().map(|(_, ty)| ty.clone()).collect(),
                        returns: Box::new(returns.clone()),
                    },
                    nullable: false,
                    errorable: false,
                };
                self.define(name.clone(), func_type);

                self.push_scope();
                for (param_name, param_type) in params {
                    self.define(param_name.clone(), param_type.clone());
                }

                let prev_return_type = self.current_return_type.clone();
                self.current_return_type = Some(returns.clone());

                let typed_body: Vec<TypedStatement> = body
                    .iter()
                    .map(|s| self.check_stmt(s))
                    .collect::<Result<_, _>>()?;

                self.current_return_type = prev_return_type;
                self.pop_scope();

                Ok(TypedStatement::Function {
                    name: name.clone(),
                    params: params.clone(),
                    returns: returns.clone(),
                    body: typed_body,
                })
            }

            ast::Statement::Struct { name, fields } => {
                self.structs
                    .insert(name.clone(), (fields.clone(), self.next_struct_index));
                self.next_struct_index += 1;
                Ok(TypedStatement::Struct {
                    name: name.clone(),
                    fields: fields.clone(),
                })
            }

            ast::Statement::Error { name } => {
                self.errors.insert(name.clone());
                // Treat as a struct with a single `message: String` field
                let fields = vec![(
                    "message".to_string(),
                    Type {
                        kind: TypeKind::String,
                        nullable: false,
                        errorable: false,
                    },
                )];
                self.structs
                    .insert(name.clone(), (fields.clone(), self.next_struct_index));
                self.next_struct_index += 1;
                Ok(TypedStatement::Struct {
                    name: name.clone(),
                    fields,
                })
            }

            ast::Statement::Produce(expr) => {
                let typed_expr = self.check_expr(expr)?;
                Ok(TypedStatement::Produce(typed_expr))
            }

            ast::Statement::Print(expr) => {
                let typed_expr = self.check_expr(expr)?;
                if typed_expr.ty.nullable || typed_expr.ty.errorable {
                    return Err(TypeError::new(
                        "Cannot print nullable or errorable expression",
                    ));
                }
                Ok(TypedStatement::Print(typed_expr))
            }

            ast::Statement::Raise(expr) => {
                let typed_expr = self.check_expr(expr)?;
                if let TypeKind::Struct { name } = &typed_expr.ty.kind {
                    if !self.errors.contains(name) {
                        return Err(TypeError::new(format!(
                            "'{}' is not an error type",
                            name
                        )));
                    }
                } else {
                    return Err(TypeError::new("Can only raise error types"));
                }
                if let Some(expected_type) = &self.current_return_type {
                    if !expected_type.errorable {
                        return Err(TypeError::new(
                            "Cannot raise in a function that does not return an errorable type",
                        ));
                    }
                } else {
                    return Err(TypeError::new("Raise statement outside of function"));
                }
                Ok(TypedStatement::Raise(typed_expr))
            }
        }
    }

    pub fn check_program(&mut self, program: &ast::Program) -> Result<TypedProgram, TypeError> {
        let typed_statements: Vec<TypedStatement> = program
            .statements
            .iter()
            .map(|s| self.check_stmt(s))
            .collect::<Result<_, _>>()?;

        Ok(TypedProgram {
            statements: typed_statements,
        })
    }
}
