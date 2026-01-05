use std::{collections::HashMap, vec};
use crate::ast::{Expr, Program, Statement, Type};

pub struct FunctionIndexer {
    pub next_index: u32,
    pub function_signatures: Vec<(String, Vec<Type>, Type)>,
}

impl FunctionIndexer {
    pub fn new() -> Self {
        FunctionIndexer {
            next_index: 1,
            function_signatures: vec![("main".to_string(), vec![], Type { kind: crate::ast::TypeKind::Primitive("integer".to_string()), nullable: true, errorable: true })],
        }
    }

    pub fn index_stmt(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Function { params, body, local_types, function_index, name, return_type, local_index } => {
                if name == "main"  {
                    if !params.is_empty() {
                        return Err("Main function must not have parameters".to_string());
                    } else if return_type != (&Type { kind: crate::ast::TypeKind::Primitive("integer".to_string()), nullable: true, errorable: true }) {
                        return Err("Main function must have int?! return type".to_string());
                    } else {
                        function_index.set(Some(0));
                        for stmt in body {
                            self.index_stmt(stmt)?;
                        }
                        return Ok(());
                    }
                }
                let func_index = self.next_index;
                self.next_index += 1;
                self.function_signatures.push((
                    name.clone(),
                    params.iter().map(|(_, ty)| ty.clone()).collect(),
                    return_type.clone(),
                ));
                function_index.set(Some(func_index));

                for stmt in body {
                    self.index_stmt(stmt)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub fn index_program(&mut self, statements: Vec<Statement>) -> Result<Program, String> {
        let mut reordered = Vec::new();
        let mut main_idx = None;
        for (i, stmt) in statements.iter().enumerate() {
            if let Statement::Function { name, .. } = stmt {
                if name == "main" {
                    main_idx = Some(i);
                    break;
                }
            }
        }

        if let Some(idx) = main_idx {
            let mut statements = statements;
            let main_stmt = statements.remove(idx);
            reordered.push(main_stmt);
            reordered.extend(statements);
        } else {
            reordered = statements;
        }

        for stmt in &reordered {
            self.index_stmt(stmt)?;
        }

        Ok(Program { statements: reordered, function_signatures: self.function_signatures.clone() })
    }
}
