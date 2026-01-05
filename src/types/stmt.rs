use crate::ast::{Statement, Type, TypeKind};
use super::{TypeChecker, TypeError};

impl TypeChecker {
    pub fn check_stmt(&mut self, stmt: &Statement) -> Result<(), TypeError> {
        match stmt {
            Statement::Expr(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
            Statement::Let { name, value, type_annotation, local_index } => {
                if let Some(init_expr) = value {
                    let init_type = self.check_expr(init_expr)?;
                    if !self.is_assignable(&init_type, type_annotation) {
                        return Err(TypeError::new(format!(
                            "Incompatible type in let binding for '{}'",
                            name
                        )));
                    }
                } else {
                    if !type_annotation.nullable {
                        return Err(TypeError::new(format!(
                            "Let binding for '{}' without initializer must be nullable",
                            name
                        )));
                    }
                }
                self.define(name.clone(), type_annotation.clone());
                Ok(())
            }
            Statement::Const { name, value, type_annotation, local_index } => {
                let value_type = self.check_expr(value)?;
                if !self.is_assignable(&value_type, type_annotation) {
                    return Err(TypeError::new(format!(
                        "Incompatible type in const binding for '{}'",
                        name
                    )));
                }
                self.define(name.clone(), type_annotation.clone());
                Ok(())
            }
            Statement::Return(expr) => {
                if let Some(ret_expr) = expr {
                    let ret_type = self.check_expr(ret_expr)?;
                    if let Some(expected_type) = &self.current_return_type {
                        if !self.is_assignable(&ret_type, expected_type) {
                            return Err(TypeError::new("Incompatible return type"));
                        }
                    } else {
                        return Err(TypeError::new("Return statement outside of function"));
                    }
                } else {
                    if let Some(expected_type) = &self.current_return_type {
                        if !expected_type.nullable {
                            return Err(TypeError::new("Incompatible return type: expected non-void"));
                        }
                    } else {
                        return Err(TypeError::new("Return statement outside of function"));
                    }
                }
                Ok(())
            }
            Statement::Break => {
                Ok(())
            }
            Statement::Continue => {
                Ok(())
            }
            Statement::If { condition, consequent, alternate } => {
                let cond_type = self.check_expr(condition)?;
                if !self.is_boolean(&cond_type) || cond_type.nullable || cond_type.errorable {
                    return Err(TypeError::new("If condition must be a non-nullable, non-errorable boolean"));
                }

                self.push_scope();
                for stmt in consequent {
                    self.check_stmt(stmt)?;
                }
                self.pop_scope();

                if let Some(alt_stmts) = alternate {
                    self.push_scope();
                    for stmt in alt_stmts {
                        self.check_stmt(stmt)?;
                    }
                    self.pop_scope();
                }

                Ok(())
            }
            Statement::For { initializer, condition, increment, body } => {
                self.push_scope();
                self.check_stmt(initializer)?;

                let cond_type = self.check_expr(condition)?;
                if !self.is_boolean(&cond_type) || cond_type.nullable || cond_type.errorable {
                    return Err(TypeError::new("For loop condition must be a non-nullable, non-errorable boolean"));
                }

                for stmt in body {
                    self.check_stmt(stmt)?;
                }

                self.check_stmt(increment)?;

                self.pop_scope();
                Ok(())
            }
            Statement::While { condition, body } => {
                let cond_type = self.check_expr(condition)?;
                if !self.is_boolean(&cond_type) || cond_type.nullable || cond_type.errorable {
                    return Err(TypeError::new("While loop condition must be a non-nullable, non-errorable boolean"));
                }

                self.push_scope();
                for stmt in body {
                    self.check_stmt(stmt)?;
                }
                self.pop_scope();

                Ok(())
            }
            Statement::Function { name, params, return_type, body, local_types, function_index, local_index } => {
                let func_type = Type {
                    kind: TypeKind::Function {
                        param_types: params.iter().map(|(_, ty)| ty.clone()).collect(),
                        return_type: Box::new(return_type.clone()),
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
                self.current_return_type = Some(return_type.clone());

                for stmt in body {
                    self.check_stmt(stmt)?;
                }

                self.current_return_type = prev_return_type;
                self.pop_scope();

                Ok(())
            }
            Statement::Struct { name, fields } => {
                self.structs.insert(name.clone(), fields.clone());
                Ok(())
            }
            Statement::Error { name } => {
                self.errors.insert(name.clone());
                Ok(())
            }
            Statement::Match { expr, arms } => {
                todo!()
            }
            Statement::Print(expr) => {
                let expr_type = self.check_expr(expr)?;
                if expr_type.nullable || expr_type.errorable {
                    return Err(TypeError::new("Cannot print nullable or errorable expression"));
                }
                Ok(())
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
