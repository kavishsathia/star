use crate::ast::aast::{self, AnalyzedExpr, AnalyzedProgram, AnalyzedStatement};
use crate::ast::{Type, TypeKind};
use crate::ast::FlattenedProgram;

pub fn segregate_fields(fields: Vec<(String, Type)>) -> (Vec<(String, Type)>, u32, u32) {
    let mut struct_ptrs = vec![];
    let mut list_ptrs = vec![];
    let mut primitives = vec![];

    for (name, ty) in fields {
        match &ty.kind {
            // TODO: function change order
            TypeKind::Struct { .. } | TypeKind::Function { .. } => struct_ptrs.push((name, ty)),
            TypeKind::List { .. } | TypeKind::String => list_ptrs.push((name, ty)),
            _ => primitives.push((name, ty)),
        }
    }

    let struct_count = struct_ptrs.len() as u32;
    let list_count = list_ptrs.len() as u32;

    struct_ptrs.append(&mut list_ptrs);
    struct_ptrs.append(&mut primitives);

    (struct_ptrs, struct_count, list_count)
}

pub struct Flattener {
    structs: Vec<(AnalyzedStatement, u32, u32)>,
    functions: Vec<AnalyzedStatement>,
    captures: Vec<(String, Type, CaptureKind)>,
}

#[derive(Debug, Clone)]
enum CaptureKind {
    Index(u32),
    Field,
}

impl Flattener {
    pub fn new() -> Self {
        Flattener {
            structs: vec![],
            functions: vec![],
            captures: vec![],
        }
    }
}

impl Flattener {
    pub fn gather_captures(
        &mut self,
        body: &Vec<AnalyzedStatement>,
    ) -> Vec<(String, Type, CaptureKind)> {
        let mut captures = vec![];
        for statement in body {
            match statement {
                AnalyzedStatement::If {
                    condition,
                    then_block,
                    else_block,
                } => {
                    captures.extend(self.gather_captures(then_block));
                    if let Some(else_blk) = else_block {
                        captures.extend(self.gather_captures(else_blk));
                    }
                }
                AnalyzedStatement::While { condition, body } => {
                    captures.extend(self.gather_captures(body));
                }
                AnalyzedStatement::For {
                    init,
                    condition,
                    update,
                    body,
                } => {
                    captures.extend(self.gather_captures(&vec![*init.clone()]));
                    captures.extend(self.gather_captures(&vec![*update.clone()]));
                    captures.extend(self.gather_captures(body));
                }
                AnalyzedStatement::Let {
                    ty,
                    captured,
                    index,
                    ..
                }
                | AnalyzedStatement::Const {
                    ty,
                    captured,
                    index,
                    ..
                } => {
                    if let Some(field_name) = captured.borrow().as_ref() {
                        captures.push((
                            field_name.clone(),
                            ty.clone(),
                            CaptureKind::Index(index.unwrap()),
                        ));
                        println!(
                            "Captured variable '{}' of type {:?} at index {:?}",
                            field_name, ty, index
                        );
                    }
                }
                AnalyzedStatement::Function {
                    name: _,
                    params,
                    returns,
                    body: _,
                    captured,
                    index,
                    fn_index: _,
                    ..
                } => {
                    if let Some(field_name) = captured.borrow().as_ref() {
                        captures.push((
                            field_name.clone(),
                            Type {
                                kind: TypeKind::Function {
                                    params: params.iter().map(|(_, t, _, _)| t.clone()).collect(),
                                    returns: Box::new(returns.clone()),
                                },
                                nullable: false,
                                errorable: false,
                            },
                            CaptureKind::Index(index.unwrap()),
                        ));
                    }
                }
                _ => {}
            }
        }
        captures
    }

    pub fn scan_params(
        &mut self,
        params: &Vec<(
            String,
            Type,
            u32,
            std::rc::Rc<std::cell::RefCell<Option<String>>>,
        )>,
    ) -> Vec<(String, Type, CaptureKind)> {
        self.captures.clear();
        for (name, ty, index, captured) in params.iter() {
            if let Some(field_name) = captured.borrow().as_ref() {
                self.captures
                    .push((field_name.clone(), ty.clone(), CaptureKind::Index(*index)));
            }
        }
        self.captures.iter().cloned().collect()
    }

    pub fn flatten_stmt(
        &mut self,
        stmt: &AnalyzedStatement,
        captures: Vec<(String, Type, CaptureKind)>,
        prev: String,
    ) -> AnalyzedStatement {
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
                let fn_captures = self.gather_captures(body);
                let param_captures = self.scan_params(params);

                let mut captures_to_pass_down = vec![];
                for (n, t, k) in captures.iter() {
                    captures_to_pass_down.push((n.clone(), t.clone(), CaptureKind::Field));
                }

                for (n, t, k) in fn_captures.iter() {
                    captures_to_pass_down.push((n.clone(), t.clone(), k.clone()));
                }

                for (n, t, k) in param_captures.iter() {
                    captures_to_pass_down.push((n.clone(), t.clone(), k.clone()));
                }

                println!("Function '{}' captures {:?}", name, fn_captures);
                println!("Function '{}' captures: {:?}", name, captures_to_pass_down);

                let (segregated, struct_count, list_count) = segregate_fields(
                    captures_to_pass_down
                        .iter()
                        .map(|(n, t, _)| (n.clone(), t.clone()))
                        .collect(),
                );
                self.structs.push((
                    AnalyzedStatement::Struct {
                        name: format!("{}", name),
                        fields: segregated,
                    },
                    struct_count,
                    list_count,
                ));

                let outer_fields: Vec<(String, AnalyzedExpr)> = captures
                    .iter()
                    .map(|(n, t, k)| {
                        (
                            n.clone(),
                            match k {
                                CaptureKind::Field => AnalyzedExpr {
                                    expr: aast::Expr::Field {
                                        object: Box::new(AnalyzedExpr {
                                            expr: aast::Expr::Identifier {
                                                name: "captures".to_string(),
                                                index: Some(1),
                                            },
                                            ty: Type {
                                                kind: TypeKind::Struct {
                                                    name: format!("{}", name),
                                                },
                                                nullable: false,
                                                errorable: false,
                                            },
                                        }),
                                        field: n.clone(),
                                    },
                                    ty: t.clone(),
                                },
                                CaptureKind::Index(idx) => AnalyzedExpr {
                                    expr: aast::Expr::Identifier {
                                        name: n.clone(),
                                        index: Some(*idx),
                                    },
                                    ty: t.clone(),
                                },
                            },
                        )
                    })
                    .collect();

                let struct_init = AnalyzedExpr {
                    expr: aast::Expr::New {
                        name: format!("{}", prev),
                        fields: outer_fields,
                    },
                    ty: Type {
                        kind: TypeKind::Struct {
                            name: format!("{}", prev),
                        },
                        nullable: false,
                        errorable: false,
                    },
                };

                let analyzed_body: Vec<_> = body
                    .iter()
                    .map(|s| self.flatten_stmt(s, captures_to_pass_down.clone(), name.clone()))
                    .collect();

                // Push the flattened function to the functions list
                self.functions.push(AnalyzedStatement::Function {
                    name: name.clone(),
                    params: params.clone(),
                    returns: returns.clone(),
                    body: analyzed_body,
                    captured: captured.clone(),
                    index: *index,
                    fn_index: *fn_index,
                    locals: locals.clone(),
                });

                let fn_type = Type {
                    kind: TypeKind::Function {
                        params: params.iter().map(|(_, t, _, _)| t.clone()).collect(),
                        returns: Box::new(returns.clone()),
                    },
                    nullable: false,
                    errorable: false,
                };

                AnalyzedStatement::LocalClosure {
                    fn_index: fn_index.unwrap(),
                    captures: Box::new(struct_init),
                    index: index.unwrap(),
                }
            }
            AnalyzedStatement::If {
                condition,
                then_block,
                else_block,
            } => {
                let analyzed_then: Vec<_> = then_block
                    .iter()
                    .map(|s| self.flatten_stmt(s, captures.clone(), prev.clone()))
                    .collect();
                let analyzed_else = else_block.as_ref().map(|stmts| {
                    let result: Vec<_> = stmts
                        .iter()
                        .map(|s| self.flatten_stmt(s, captures.clone(), prev.clone()))
                        .collect();
                    result
                });
                AnalyzedStatement::If {
                    condition: condition.clone(),
                    then_block: analyzed_then,
                    else_block: analyzed_else,
                }
            }
            AnalyzedStatement::While { condition, body } => {
                let analyzed_body: Vec<_> = body
                    .iter()
                    .map(|s| self.flatten_stmt(s, captures.clone(), prev.clone()))
                    .collect();
                AnalyzedStatement::While {
                    condition: condition.clone(),
                    body: analyzed_body,
                }
            }
            AnalyzedStatement::For {
                init,
                condition,
                update,
                body,
            } => {
                let analyzed_init =
                    Box::new(self.flatten_stmt(init, captures.clone(), prev.clone()));
                let analyzed_update =
                    Box::new(self.flatten_stmt(update, captures.clone(), prev.clone()));
                let analyzed_body: Vec<_> = body
                    .iter()
                    .map(|s| self.flatten_stmt(s, captures.clone(), prev.clone()))
                    .collect();
                AnalyzedStatement::For {
                    init: analyzed_init,
                    condition: condition.clone(),
                    update: analyzed_update,
                    body: analyzed_body,
                }
            }
            AnalyzedStatement::Struct { name, fields } => {
                let (segregated, struct_count, list_count) = segregate_fields(fields.clone());
                let str = AnalyzedStatement::Struct {
                    name: name.clone(),
                    fields: segregated,
                };

                self.structs.push((str.clone(), struct_count, list_count));
                str
            }
            nonfunc => nonfunc.clone(),
        }
    }

    pub fn flatten_program(&mut self, program: &AnalyzedProgram) -> FlattenedProgram {
        for stmt in &program.statements {
            self.flatten_stmt(stmt, vec![], "root".to_string());
        }

        let structs = self.structs.drain(..).collect::<Vec<_>>();
        let mut functions = self.functions.drain(..).collect::<Vec<_>>();

        if let Some(pos) = functions
            .iter()
            .position(|f| matches!(f, AnalyzedStatement::Function { name, .. } if name == "main"))
        {
            let main_fn = functions.remove(pos);
            functions.insert(0, main_fn);
        }

        FlattenedProgram { structs, functions }
    }
}
