use std::{cell::RefCell, collections::HashMap, rc::Rc};
use crate::tast::{self, TypedExpr, TypedProgram, TypedStatement};
use crate::aast::{self, AnalyzedExpr, AnalyzedProgram, AnalyzedStatement};
use crate::ast::{Type, TypeKind};

pub struct LocalsIndexer {
    scopes: Vec<Vec<HashMap<String, (u32, Rc<RefCell<Option<String>>>)>>>,
    locals_types_stack: Vec<Vec<Type>>,
    pub fn_count: u32,
    free_var_count: u32,
    fn_names: Vec<String>,
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
            fn_count: 0,
            free_var_count: 0,
            fn_names: vec![],
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

    pub fn define(&mut self, name: String, typ: Type, captured: Rc<RefCell<Option<String>>>) -> u32 {
        if let Some(scopes) = self.scopes.last_mut() {
            if let Some(current_scope) = scopes.last_mut() {
                let locals = self.locals_types_stack.last_mut().unwrap();
                let index = locals.len() as u32 + 2; // first two reserved for tmp var and capture pointer

                if current_scope.contains_key(&name) {
                    panic!("Local variable '{}' already defined in this scope", name);
                }

                current_scope.insert(name, (index, captured));
                locals.push(typ);
                return index;
            }
        }
        panic!("No function scope available to define local variable");
    }

    pub fn lookup(&mut self, name: &str) -> VariableKind {
        if let Some(scope) = self.scopes.last() {
            for local_scope in scope.iter().rev() {
                if let Some(index) = local_scope.get(name) {
                    return VariableKind::Local(index.0);
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
                        return VariableKind::Captured(field);
                    } else {
                        return VariableKind::Captured(borrowed.as_ref().unwrap().clone());
                    }
                }
            }
        }
        panic!("Undefined local variable '{}'", name);
    }

    pub fn analyze_stmt(&mut self, stmt: &TypedStatement) -> AnalyzedStatement {
        match stmt {
            TypedStatement::Let { name, ty, value } => {
                let analyzed_value = value.as_ref().map(|e| self.analyze_expr(e));
                let captured = Rc::new(RefCell::new(None));
                let index = self.define(name.clone(), ty.clone(), Rc::clone(&captured));
                AnalyzedStatement::Let {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: analyzed_value,
                    captured,
                    index: Some(index),
                }
            }
            TypedStatement::Const { name, ty, value } => {
                let analyzed_value = self.analyze_expr(value);
                let captured = Rc::new(RefCell::new(None));
                let index = self.define(name.clone(), ty.clone(), Rc::clone(&captured));
                AnalyzedStatement::Const {
                    name: name.clone(),
                    ty: ty.clone(),
                    value: analyzed_value,
                    captured,
                    index: Some(index),
                }
            }
            TypedStatement::Function { name, params, returns, body } => {
                self.push_fn(name.clone());

                let mut analyzed_params = vec![];
                for (param_name, param_type) in params {
                    let captured = Rc::new(RefCell::new(None));
                    let index = self.define(param_name.clone(), param_type.clone(), Rc::clone(&captured));
                    analyzed_params.push((param_name.clone(), param_type.clone(), index, captured));
                }

                let mut analyzed_body = vec![];
                for stmt in body {
                    analyzed_body.push(self.analyze_stmt(stmt));
                }

                let locals = self.pop_fn();

                let captured = Rc::new(RefCell::new(None));
                let index = self.define(name.clone(), Type {
                    kind: TypeKind::Function {
                        params: params.iter().map(|(_, ty)| ty.clone()).collect(),
                        returns: Box::new(returns.clone()),
                    },
                    nullable: false,
                    errorable: false,
                }, Rc::clone(&captured));

                let fn_index = self.fn_count;
                self.fn_count += 1;

                AnalyzedStatement::Function {
                    name: name.clone(),
                    params: analyzed_params,
                    returns: returns.clone(),
                    body: analyzed_body,
                    captured,
                    index: Some(index),
                    fn_index: Some(fn_index),
                    locals,
                }
            }
            TypedStatement::If { condition, then_block, else_block } => {
                let analyzed_condition = self.analyze_expr(condition);
                self.push_scope();
                let analyzed_then: Vec<_> = then_block.iter().map(|s| self.analyze_stmt(s)).collect();
                self.pop_scope();
                let analyzed_else = else_block.as_ref().map(|stmts| {
                    self.push_scope();
                    let result: Vec<_> = stmts.iter().map(|s| self.analyze_stmt(s)).collect();
                    self.pop_scope();
                    result
                });
                AnalyzedStatement::If {
                    condition: analyzed_condition,
                    then_block: analyzed_then,
                    else_block: analyzed_else,
                }
            }
            TypedStatement::While { condition, body } => {
                let analyzed_condition = self.analyze_expr(condition);
                self.push_scope();
                let analyzed_body: Vec<_> = body.iter().map(|s| self.analyze_stmt(s)).collect();
                self.pop_scope();
                AnalyzedStatement::While {
                    condition: analyzed_condition,
                    body: analyzed_body,
                }
            }
            TypedStatement::For { init, condition, update, body } => {
                self.push_scope();
                let analyzed_init = Box::new(self.analyze_stmt(init));
                let analyzed_condition = self.analyze_expr(condition);
                let analyzed_update = Box::new(self.analyze_stmt(update));
                let analyzed_body: Vec<_> = body.iter().map(|s| self.analyze_stmt(s)).collect();
                self.pop_scope();
                AnalyzedStatement::For {
                    init: analyzed_init,
                    condition: analyzed_condition,
                    update: analyzed_update,
                    body: analyzed_body,
                }
            }
            TypedStatement::Expr(expr) => {
                AnalyzedStatement::Expr(self.analyze_expr(expr))
            }
            TypedStatement::Return(expr) => {
                AnalyzedStatement::Return(expr.as_ref().map(|e| self.analyze_expr(e)))
            }
            TypedStatement::Print(expr) => {
                AnalyzedStatement::Print(self.analyze_expr(expr))
            }
            TypedStatement::Produce(expr) => {
                AnalyzedStatement::Produce(self.analyze_expr(expr))
            }
            TypedStatement::Break => AnalyzedStatement::Break,
            TypedStatement::Continue => AnalyzedStatement::Continue,
            TypedStatement::Struct { name, fields } => {
                AnalyzedStatement::Struct {
                    name: name.clone(),
                    fields: fields.clone(),
                }
            }
            TypedStatement::Error { name } => {
                AnalyzedStatement::Error { name: name.clone() }
            }
        }
    }

    fn analyze_expr(&mut self, expr: &TypedExpr) -> AnalyzedExpr {
        let analyzed = match &expr.expr {
            tast::Expr::Identifier(name) => {
                match self.lookup(name) {
                    VariableKind::Local(index) => aast::Expr::Identifier {
                        name: name.clone(),
                        index: Some(index),
                    },
                    VariableKind::Captured(field) => aast::Expr::Field {
                        object: Box::new(AnalyzedExpr{
                            expr: aast::Expr::Identifier {
                            name: "captured".to_string(),
                            index: Some(1),
                        }, ty: Type {
                            kind: TypeKind::Struct { name: self.fn_names[self.fn_names.len() - 2].to_string() },
                            nullable: false,
                            errorable: false,
                        }}),
                        field,
                    },
                }
            }
            tast::Expr::Binary { left, op, right } => {
                aast::Expr::Binary {
                    left: Box::new(self.analyze_expr(left)),
                    op: op.clone(),
                    right: Box::new(self.analyze_expr(right)),
                }
            }
            tast::Expr::Unary { op, expr } => {
                aast::Expr::Unary {
                    op: op.clone(),
                    expr: Box::new(self.analyze_expr(expr)),
                }
            }
            tast::Expr::Call { callee, args } => {
                aast::Expr::Call {
                    callee: Box::new(self.analyze_expr(callee)),
                    args: args.iter().map(|a| self.analyze_expr(a)).collect(),
                }
            }
            tast::Expr::List(items) => {
                aast::Expr::List(items.iter().map(|i| self.analyze_expr(i)).collect())
            }
            tast::Expr::Field { object, field } => {
                aast::Expr::Field {
                    object: Box::new(self.analyze_expr(object)),
                    field: field.clone(),
                }
            }
            tast::Expr::Index { object, key } => {
                aast::Expr::Index {
                    object: Box::new(self.analyze_expr(object)),
                    key: Box::new(self.analyze_expr(key)),
                }
            }
            tast::Expr::New { name, fields } => {
                aast::Expr::New {
                    name: name.clone(),
                    fields: fields.iter().map(|(n, e)| (n.clone(), self.analyze_expr(e))).collect(),
                }
            }
            tast::Expr::Match { expr, binding, arms } => {
                aast::Expr::Match {
                    expr: Box::new(self.analyze_expr(expr)),
                    binding: binding.clone(),
                    arms: arms.iter().map(|(pattern, stmts)| {
                        self.push_scope();
                        let analyzed_stmts: Vec<_> = stmts.iter().map(|s| self.analyze_stmt(s)).collect();
                        self.pop_scope();
                        (pattern.clone(), analyzed_stmts)
                    }).collect(),
                }
            }
            tast::Expr::UnwrapError(e) => {
                aast::Expr::UnwrapError(Box::new(self.analyze_expr(e)))
            }
            tast::Expr::UnwrapNull(e) => {
                aast::Expr::UnwrapNull(Box::new(self.analyze_expr(e)))
            }
            tast::Expr::Null => aast::Expr::Null,
            tast::Expr::Integer(n) => aast::Expr::Integer(*n),
            tast::Expr::Float(n) => aast::Expr::Float(*n),
            tast::Expr::String(s) => aast::Expr::String(s.clone()),
            tast::Expr::Boolean(b) => aast::Expr::Boolean(*b),
        };
        AnalyzedExpr {
            expr: analyzed,
            ty: expr.ty.clone(),
        }
    }

    pub fn analyze_program(&mut self, program: &TypedProgram) -> AnalyzedProgram {
        let mut statements: Vec<_> = program.statements.iter().collect();
        if let Some(main_idx) = statements.iter().position(|s| {
            matches!(s, TypedStatement::Function { name, .. } if name == "main")
        }) {
            let main_fn = statements.remove(main_idx);
            statements.insert(0, main_fn);
        }

        self.push_fn("root".to_string());
        let analyzed: Vec<_> = statements.iter().map(|s| self.analyze_stmt(s)).collect();
        self.pop_fn();
        AnalyzedProgram { statements: analyzed }
    }
}
