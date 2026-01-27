use crate::ast::aast::{self, AnalyzedExpr, AnalyzedProgram, AnalyzedStatement};
use crate::ast::{Type, TypeKind};
use crate::error::CompilerError;
use crate::ast::tast::{self, TypedExpr, TypedProgram, TypedStatement};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

pub struct LocalsIndexer {
    scopes: Vec<Vec<HashMap<String, (u32, Rc<RefCell<Option<String>>>)>>>,
    locals_types_stack: Vec<Vec<Type>>,
    pub fn_count: u32,
    free_var_count: u32,
    fn_names: Vec<String>,
    current_param_count: u32,
}

enum VariableKind {
    Local(u32),
    Captured(String),
}

impl LocalsIndexer {
    pub fn new() -> Self {
        LocalsIndexer {
            scopes: vec![],
            locals_types_stack: vec![],
            fn_count: 1,
            free_var_count: 0,
            fn_names: vec![],
            current_param_count: 0,
        }
    }

    pub fn push_fn(&mut self, name: String) {
        self.scopes.push(vec![HashMap::new()]);
        self.locals_types_stack.push(vec![]);
        self.fn_names.push(name);
    }

    pub fn pop_fn(&mut self) -> Vec<Type> {
        self.scopes.pop();
        self.fn_names.pop();
        self.current_param_count = 0;
        self.locals_types_stack.pop().unwrap_or_default()
    }

    pub fn push_scope(&mut self) {
        if let Some(scopes) = self.scopes.last_mut() {
            scopes.push(HashMap::new());
        }
    }

    pub fn pop_scope(&mut self) {
        if let Some(scopes) = self.scopes.last_mut() {
            scopes.pop();
        }
    }

    pub fn define_param(
        &mut self,
        name: String,
        param_index: u32,
        captured: Rc<RefCell<Option<String>>>,
    ) -> Result<u32, CompilerError> {
        if let Some(scopes) = self.scopes.last_mut() {
            if let Some(current_scope) = scopes.last_mut() {
                let index = param_index + 3; // first three reserved for 2 tmp vars and capture pointer

                if current_scope.contains_key(&name) {
                    return Err(CompilerError::Locals {
                        message: format!("Parameter '{}' already defined in this scope", name),
                    });
                }

                current_scope.insert(name, (index, captured));
                // Don't push to locals_types_stack - params are already in WASM signature
                return Ok(index);
            }
        }
        Err(CompilerError::Locals {
            message: "No function scope available to define parameter".to_string(),
        })
    }

    pub fn define(
        &mut self,
        name: String,
        typ: Type,
        captured: Rc<RefCell<Option<String>>>,
    ) -> Result<u32, CompilerError> {
        if let Some(scopes) = self.scopes.last_mut() {
            if let Some(current_scope) = scopes.last_mut() {
                let locals = self.locals_types_stack.last_mut().unwrap();
                let num_params = self.current_param_count;
                let index = locals.len() as u32 + 3 + num_params; // offset by params

                if current_scope.contains_key(&name) {
                    return Err(CompilerError::Locals {
                        message: format!("Local variable '{}' already defined in this scope", name),
                    });
                }

                current_scope.insert(name, (index, captured));
                locals.push(typ);
                return Ok(index);
            }
        }
        Err(CompilerError::Locals {
            message: "No function scope available to define local variable".to_string(),
        })
    }

    pub fn lookup(&mut self, name: &str) -> Result<VariableKind, CompilerError> {
        if let Some(scope) = self.scopes.last() {
            for local_scope in scope.iter().rev() {
                if let Some(index) = local_scope.get(name) {
                    return Ok(VariableKind::Local(index.0));
                }
            }
        }
        for fn_scope in self.scopes.iter().rev().skip(1) {
            for local_scope in fn_scope.iter().rev() {
                if let Some((_, captured)) = local_scope.get(name) {
                    let mut borrowed = captured.borrow_mut();
                    if borrowed.is_none() {
                        let field = format!("field{}", self.free_var_count);
                        self.free_var_count += 1;
                        *borrowed = Some(field.clone());
                        return Ok(VariableKind::Captured(field));
                    } else {
                        return Ok(VariableKind::Captured(borrowed.as_ref().unwrap().clone()));
                    }
                }
            }
        }
        Err(CompilerError::Locals {
            message: format!("Undefined local variable '{}'", name),
        })
    }

    pub fn analyze_stmt(&mut self, stmt: &TypedStatement) -> Result<AnalyzedStatement, CompilerError> {
        match stmt {
            TypedStatement::Let { name, ty, value } => {
                let analyzed_value = match value {
                    Some(e) => Some(self.analyze_expr(e)?),
                    None => None,
                };
                let captured = Rc::new(RefCell::new(None));
                let index = self.define(name.clone(), ty.clone(), Rc::clone(&captured))?;
                Ok(AnalyzedStatement::Let {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: analyzed_value,
                    captured,
                    index: Some(index),
                })
            }
            TypedStatement::Const { name, ty, value } => {
                let analyzed_value = self.analyze_expr(value)?;
                let captured = Rc::new(RefCell::new(None));
                let index = self.define(name.clone(), ty.clone(), Rc::clone(&captured))?;
                Ok(AnalyzedStatement::Const {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: analyzed_value,
                    captured,
                    index: Some(index),
                })
            }
            TypedStatement::Function {
                name,
                params,
                returns,
                body,
            } => {
                let captured = Rc::new(RefCell::new(None));
                let index = self.define(
                    name.clone(),
                    Type {
                        kind: TypeKind::Function {
                            params: params.iter().map(|(_, ty)| ty.clone()).collect(),
                            returns: Box::new(returns.clone()),
                        },
                        nullable: false,
                        errorable: false,
                    },
                    Rc::clone(&captured),
                )?;

                self.push_fn(name.clone());
                self.current_param_count = params.len() as u32;

                let mut analyzed_params = vec![];
                for (i, (param_name, param_type)) in params.iter().enumerate() {
                    let captured = Rc::new(RefCell::new(None));
                    let index =
                        self.define_param(param_name.clone(), i as u32, Rc::clone(&captured))?;
                    analyzed_params.push((param_name.clone(), param_type.clone(), index, captured));
                }

                let mut analyzed_body = vec![];
                for stmt in body {
                    analyzed_body.push(self.analyze_stmt(stmt)?);
                }

                let locals = self.pop_fn();

                let mut fn_index = self.fn_count;
                if name == "main" {
                    fn_index = 0;
                }
                self.fn_count += 1;

                Ok(AnalyzedStatement::Function {
                    name: name.clone(),
                    params: analyzed_params,
                    returns: returns.clone(),
                    body: analyzed_body,
                    captured,
                    index: Some(index),
                    fn_index: Some(fn_index),
                    locals,
                })
            }
            TypedStatement::If {
                condition,
                then_block,
                else_block,
            } => {
                let analyzed_condition = self.analyze_expr(condition)?;
                self.push_scope();
                let mut analyzed_then = Vec::new();
                for s in then_block {
                    analyzed_then.push(self.analyze_stmt(s)?);
                }
                self.pop_scope();
                let analyzed_else = match else_block {
                    Some(stmts) => {
                        self.push_scope();
                        let mut result = Vec::new();
                        for s in stmts {
                            result.push(self.analyze_stmt(s)?);
                        }
                        self.pop_scope();
                        Some(result)
                    }
                    None => None,
                };
                Ok(AnalyzedStatement::If {
                    condition: analyzed_condition,
                    then_block: analyzed_then,
                    else_block: analyzed_else,
                })
            }
            TypedStatement::While { condition, body } => {
                let analyzed_condition = self.analyze_expr(condition)?;
                self.push_scope();
                let mut analyzed_body = Vec::new();
                for s in body {
                    analyzed_body.push(self.analyze_stmt(s)?);
                }
                self.pop_scope();
                Ok(AnalyzedStatement::While {
                    condition: analyzed_condition,
                    body: analyzed_body,
                })
            }
            TypedStatement::For {
                init,
                condition,
                update,
                body,
            } => {
                self.push_scope();
                let analyzed_init = Box::new(self.analyze_stmt(init)?);
                let analyzed_condition = self.analyze_expr(condition)?;
                let analyzed_update = Box::new(self.analyze_stmt(update)?);
                let mut analyzed_body = Vec::new();
                for s in body {
                    analyzed_body.push(self.analyze_stmt(s)?);
                }
                self.pop_scope();
                Ok(AnalyzedStatement::For {
                    init: analyzed_init,
                    condition: analyzed_condition,
                    update: analyzed_update,
                    body: analyzed_body,
                })
            }
            TypedStatement::Expr(expr) => Ok(AnalyzedStatement::Expr(self.analyze_expr(expr)?)),
            TypedStatement::Return(expr) => {
                let analyzed = match expr {
                    Some(e) => Some(self.analyze_expr(e)?),
                    None => None,
                };
                Ok(AnalyzedStatement::Return(analyzed))
            }
            TypedStatement::Print(expr) => Ok(AnalyzedStatement::Print(self.analyze_expr(expr)?)),
            TypedStatement::Produce(expr) => Ok(AnalyzedStatement::Produce(self.analyze_expr(expr)?)),
            TypedStatement::Break => Ok(AnalyzedStatement::Break),
            TypedStatement::Continue => Ok(AnalyzedStatement::Continue),
            TypedStatement::Struct { name, fields } => Ok(AnalyzedStatement::Struct {
                name: name.clone(),
                fields: fields.clone(),
            }),
            TypedStatement::Error { name } => Ok(AnalyzedStatement::Error { name: name.clone() }),
            TypedStatement::Raise(expr) => Ok(AnalyzedStatement::Raise(self.analyze_expr(expr)?)),
        }
    }

    fn analyze_expr(&mut self, expr: &TypedExpr) -> Result<AnalyzedExpr, CompilerError> {
        let analyzed = match &expr.expr {
            tast::Expr::Identifier(name) => match self.lookup(name)? {
                VariableKind::Local(index) => aast::Expr::Identifier {
                    name: name.clone(),
                    index: Some(index),
                },
                VariableKind::Captured(field) => aast::Expr::Field {
                    object: Box::new(AnalyzedExpr {
                        expr: aast::Expr::Identifier {
                            name: "captured".to_string(),
                            index: Some(2),
                        },
                        ty: Type {
                            kind: TypeKind::Struct {
                                name: self.fn_names[self.fn_names.len() - 2].to_string(),
                            },
                            nullable: false,
                            errorable: false,
                        },
                    }),
                    field,
                },
            },
            tast::Expr::Binary { left, op, right } => aast::Expr::Binary {
                left: Box::new(self.analyze_expr(left)?),
                op: op.clone(),
                right: Box::new(self.analyze_expr(right)?),
            },
            tast::Expr::Unary { op, expr } => aast::Expr::Unary {
                op: op.clone(),
                expr: Box::new(self.analyze_expr(expr)?),
            },
            tast::Expr::Call { callee, args } => {
                let mut analyzed_args = Vec::new();
                for a in args {
                    analyzed_args.push(self.analyze_expr(a)?);
                }
                aast::Expr::Call {
                    callee: Box::new(self.analyze_expr(callee)?),
                    args: analyzed_args,
                }
            }
            tast::Expr::List(items) => {
                let mut analyzed_items = Vec::new();
                for i in items {
                    analyzed_items.push(self.analyze_expr(i)?);
                }
                aast::Expr::List(analyzed_items)
            }
            tast::Expr::Field { object, field } => aast::Expr::Field {
                object: Box::new(self.analyze_expr(object)?),
                field: field.clone(),
            },
            tast::Expr::Index { object, key } => aast::Expr::Index {
                object: Box::new(self.analyze_expr(object)?),
                key: Box::new(self.analyze_expr(key)?),
            },
            tast::Expr::Slice { expr, start, end } => aast::Expr::Slice {
                expr: Box::new(self.analyze_expr(expr)?),
                start: Box::new(self.analyze_expr(start)?),
                end: Box::new(self.analyze_expr(end)?),
            },
            tast::Expr::New { name, fields } => {
                let mut analyzed_fields = Vec::new();
                for (n, e) in fields {
                    analyzed_fields.push((n.clone(), self.analyze_expr(e)?));
                }
                aast::Expr::New {
                    name: name.clone(),
                    fields: analyzed_fields,
                }
            }
            tast::Expr::Match {
                expr,
                binding,
                arms,
            } => {
                let mut analyzed_arms = Vec::new();
                for (pattern, stmts) in arms {
                    self.push_scope();
                    let mut analyzed_stmts = Vec::new();
                    for s in stmts {
                        analyzed_stmts.push(self.analyze_stmt(s)?);
                    }
                    self.pop_scope();
                    analyzed_arms.push((pattern.clone(), analyzed_stmts));
                }
                aast::Expr::Match {
                    expr: Box::new(self.analyze_expr(expr)?),
                    binding: binding.clone(),
                    arms: analyzed_arms,
                }
            }
            tast::Expr::UnwrapError(e) => aast::Expr::UnwrapError(Box::new(self.analyze_expr(e)?)),
            tast::Expr::UnwrapNull(e) => aast::Expr::UnwrapNull(Box::new(self.analyze_expr(e)?)),
            tast::Expr::Null => aast::Expr::Null,
            tast::Expr::Integer(n) => aast::Expr::Integer(*n),
            tast::Expr::Float(n) => aast::Expr::Float(*n),
            tast::Expr::String(s) => aast::Expr::String(s.clone()),
            tast::Expr::Boolean(b) => aast::Expr::Boolean(*b),
        };
        Ok(AnalyzedExpr {
            expr: analyzed,
            ty: expr.ty.clone(),
        })
    }

    pub fn analyze_program(&mut self, program: &TypedProgram) -> Result<AnalyzedProgram, CompilerError> {
        let mut statements: Vec<_> = program.statements.iter().collect();
        if let Some(main_idx) = statements
            .iter()
            .position(|s| matches!(s, TypedStatement::Function { name, .. } if name == "main"))
        {
            let main_fn = statements.remove(main_idx);
            statements.insert(0, main_fn);
        }

        self.push_fn("root".to_string());
        let mut analyzed = Vec::new();
        for s in &statements {
            analyzed.push(self.analyze_stmt(s)?);
        }
        self.pop_fn();
        Ok(AnalyzedProgram {
            statements: analyzed,
        })
    }
}
