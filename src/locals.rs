use std::collections::HashMap;
use crate::ast::{Expr, Statement, Type, TypeKind};

pub struct LocalsIndexer {
    scopes: Vec<HashMap<String, u32>>,
    pub next_index: u32,
    pub local_types: Vec<Type>,
}

impl LocalsIndexer {
    pub fn new() -> Self {
        LocalsIndexer {
            scopes: vec![HashMap::new()],
            next_index: 0,
            local_types: Vec::new(),
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: String, typ: Type) -> u32 {
        let index = self.next_index;
        self.next_index += 1;
        self.local_types.push(typ);
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, index);
        }
        index
    }

    pub fn lookup(&self, name: &str) -> Option<u32> {
        for scope in self.scopes.iter().rev() {
            if let Some(&index) = scope.get(name) {
                return Some(index);
            }
        }
        None
    }

    pub fn index_stmt(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let { name, value, type_annotation, local_index } => {
                if let Some(expr) = value {
                    self.index_expr(expr)?;
                }
                local_index.set(Some(self.define(name.clone(), type_annotation.clone())));
                Ok(())
            }
            Statement::Const { name, value, type_annotation, local_index } => {
                self.index_expr(value)?;
                local_index.set(Some(self.define(name.clone(), type_annotation.clone())));
                Ok(())
            }
            Statement::Function { name, params, body, local_types, local_index, return_type, .. } => {
                let mut fn_indexer = LocalsIndexer::new();
                for (param_name, param_type) in params {
                    fn_indexer.define(param_name.clone(), param_type.clone());
                }
                for stmt in body {
                    fn_indexer.index_stmt(stmt)?;
                }
                *local_types.borrow_mut() = fn_indexer.local_types;
                print!("Function '{}' has locals:\n", name);
                local_index.set(Some(self.define(name.clone(), Type {
                    kind: TypeKind::Function {
                        param_types: params.iter().map(|(_, ty)| ty.clone()).collect(),
                        return_type: Box::new(return_type.clone()),
                    },
                    nullable: false,
                    errorable: false,
                })));
                Ok(())
            }
            Statement::If { condition, consequent, alternate } => {
                self.index_expr(condition)?;
                self.push_scope();
                for stmt in consequent {
                    self.index_stmt(stmt)?;
                }
                self.pop_scope();
                if let Some(alt) = alternate {
                    self.push_scope();
                    for stmt in alt {
                        self.index_stmt(stmt)?;
                    }
                    self.pop_scope();
                }
                Ok(())
            }
            Statement::While { condition, body } => {
                self.index_expr(condition)?;
                self.push_scope();
                for stmt in body {
                    self.index_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Statement::For { initializer, condition, increment, body } => {
                self.push_scope();
                self.index_stmt(initializer)?;
                self.index_expr(condition)?;
                self.index_stmt(increment)?;
                for stmt in body {
                    self.index_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Statement::Expr(expr) => self.index_expr(expr),
            Statement::Return(expr) => {
                if let Some(e) = expr {
                    self.index_expr(e)?;
                }
                Ok(())
            }
            Statement::Print(expr) => self.index_expr(expr),
            Statement::Match { expr, arms } => {
                self.index_expr(expr)?;
                for (_, stmts) in arms {
                    self.push_scope();
                    for stmt in stmts {
                        self.index_stmt(stmt)?;
                    }
                    self.pop_scope();
                }
                Ok(())
            }
            Statement::Break | Statement::Continue | Statement::Struct { .. } | Statement::Error { .. } => Ok(()),
        }
    }

    fn index_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Identifier { name, local_index } => {
                if let Some(index) = self.lookup(name.as_str()) {
                    local_index.set(Some(index));
                    Ok(())
                } else {
                    Err(format!("Undefined identifier '{}'", name))
                }
            }
            Expr::Binary { left, right, .. } => {
                self.index_expr(left)?;
                self.index_expr(right)
            }
            Expr::Unary { expr, .. } => self.index_expr(expr),
            Expr::Call { callee, args } => {
                self.index_expr(callee)?;
                for arg in args {
                    self.index_expr(arg)?;
                }
                Ok(())
            }
            Expr::List(items) => {
                for item in items {
                    self.index_expr(item)?;
                }
                Ok(())
            }
            Expr::Dict(pairs) => {
                for (k, v) in pairs {
                    self.index_expr(k)?;
                    self.index_expr(v)?;
                }
                Ok(())
            }
            Expr::MemberAccess { object, .. } => self.index_expr(object),
            Expr::KeyAccess { dict, key } => {
                self.index_expr(dict)?;
                self.index_expr(key)
            }
            Expr::Init { fields, .. } => {
                for (_, expr) in fields {
                    self.index_expr(expr)?;
                }
                Ok(())
            }
            Expr::NotNull(e) | Expr::NotError(e) | Expr::NotNullOrError(e) => self.index_expr(e),
            Expr::Null | Expr::Integer(_) | Expr::Float(_) | Expr::String(_) | Expr::Boolean(_) => Ok(()),
        }
    }
}
