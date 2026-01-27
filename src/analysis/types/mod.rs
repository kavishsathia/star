mod expr;
mod stmt;

use crate::ast::{Type, TypeKind};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub struct TypeError {
    pub message: String,
}

impl TypeError {
    pub fn new(message: impl Into<String>) -> Self {
        TypeError {
            message: message.into(),
        }
    }
}

pub struct TypeChecker {
    scopes: Vec<HashMap<String, Type>>,
    pub structs: HashMap<String, (Vec<(String, Type)>, i32)>,
    pub errors: HashSet<String>,
    pub next_struct_index: i32,
    pub current_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        TypeChecker {
            scopes: vec![HashMap::new()],
            structs: HashMap::new(),
            errors: HashSet::new(),
            current_return_type: None,
            next_struct_index: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    pub fn define(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, ty);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.get(name) {
                return Some(ty);
            }
        }
        None
    }

    pub fn types_equal(&self, a: &Type, b: &Type) -> bool {
        return a == b;
    }

    pub fn is_assignable(&self, from: &Type, to: &Type) -> bool {
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

    pub fn is_numeric(&self, ty: &Type) -> bool {
        matches!(ty.kind, TypeKind::Integer | TypeKind::Float)
    }

    pub fn is_boolean(&self, ty: &Type) -> bool {
        matches!(ty.kind, TypeKind::Boolean)
    }
}
