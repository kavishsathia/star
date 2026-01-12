use crate::ast::{BinaryOp, Type, TypeKind, UnaryOp};
use crate::ir::{IRExpr, IRExprKind, IRFunction, IRProgram, IRStmt, IRStruct};
use wasm_encoder::{
    CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemArg, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

pub struct Codegen {}

impl Codegen {
    fn type_to_valtype(ty: &Type) -> ValType {
        if ty.nullable || ty.errorable {
            return ValType::I32;
        }
        match &ty.kind {
            TypeKind::Function { .. } => ValType::I64,
            TypeKind::List { .. } => ValType::I32,
            TypeKind::Struct { .. } => ValType::I32,
            _ => ValType::I64,
        }
    }
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {}
    }

    const IMPORT_COUNT: u32 = 6;

    pub fn compile(&mut self, program: &IRProgram) -> Vec<u8> {
        let mut module = Module::new();

        let mut types = TypeSection::new();
        types.ty().function(vec![ValType::I64], vec![]); // 0: print_i64
        types.ty().function(vec![], vec![]); // 1: init
        types.ty().function(vec![ValType::I32], vec![]); // 2: register
        types.ty().function(vec![ValType::I32], vec![ValType::I32]); // 3: falloc
        types.ty().function(vec![], vec![]); // 4: dinit
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32], vec![ValType::I32]); // 5: dalloc
        for func in &program.functions {
            let mut params: Vec<ValType> = vec![ValType::I32, ValType::I64, ValType::I32];
            params.extend(func.params.iter().map(Self::type_to_valtype));
            let results: Vec<ValType> = vec![Self::type_to_valtype(&func.returns)];
            types.ty().function(params, results);
        }
        module.section(&types);

        let mut imports = ImportSection::new();
        imports.import("env", "print_i64", EntityType::Function(0));
        imports.import("alloc", "init", EntityType::Function(1));
        imports.import("alloc", "register", EntityType::Function(2));
        imports.import("alloc", "falloc", EntityType::Function(3));
        imports.import("dalloc", "dinit", EntityType::Function(4));
        imports.import("dalloc", "dalloc", EntityType::Function(5));
        imports.import(
            "alloc",
            "memory",
            EntityType::Memory(wasm_encoder::MemoryType {
                minimum: 1,
                maximum: None,
                memory64: false,
                shared: false,
                page_size_log2: None,
            }),
        );
        imports.import(
            "dalloc",
            "memory",
            EntityType::Memory(wasm_encoder::MemoryType {
                minimum: 16,
                maximum: None,
                memory64: false,
                shared: false,
                page_size_log2: None,
            }),
        );
        module.section(&imports);

        let mut functions = FunctionSection::new();
        for (i, _) in program.functions.iter().enumerate() {
            functions.function((i as u32 + Self::IMPORT_COUNT) as u32);
        }
        module.section(&functions);

        if !program.functions.is_empty() {
            let mut tables = TableSection::new();
            tables.table(TableType {
                element_type: RefType::FUNCREF,
                minimum: program.functions.len() as u64,
                maximum: Some(program.functions.len() as u64),
                table64: false,
                shared: false,
            });
            module.section(&tables);
        }

        let mut exports = ExportSection::new();
        exports.export("main", wasm_encoder::ExportKind::Func, Self::IMPORT_COUNT);
        module.section(&exports);

        if !program.functions.is_empty() {
            let func_indices: Vec<u32> = (Self::IMPORT_COUNT
                ..(Self::IMPORT_COUNT + program.functions.len() as u32))
                .collect();
            let mut elements = ElementSection::new();
            elements.active(
                Some(0),
                &ConstExpr::i32_const(0),
                Elements::Functions(std::borrow::Cow::Borrowed(&func_indices)),
            );
            module.section(&elements);
        }

        let mut codes = CodeSection::new();

        for func in &program.functions {
            self.compile_function(func, &mut codes, program);
        }

        module.section(&codes);

        module.finish()
    }

    fn compile_function(
        &mut self,
        func: &IRFunction,
        codes: &mut CodeSection,
        program: &IRProgram,
    ) {
        let mut locals: Vec<(u32, ValType)> = vec![];
        locals.extend(func.locals.iter().map(|t| (1, Self::type_to_valtype(t))));
        println!("Locals: {:?}", locals.clone());
        let mut f = Function::new(locals);

        if func.name == "main" {
            f.instruction(&Instruction::Call(1)); // alloc::init
            f.instruction(&Instruction::Call(4)); // dalloc::dinit
            for ir_struct in &program.structs {
                f.instruction(&Instruction::I32Const(ir_struct.size as i32));
                f.instruction(&Instruction::Call(2));
            }
        }

        for stmt in &func.body {
            self.compile_stmt(stmt, &mut f);
        }

        f.instruction(&Instruction::End);
        codes.function(&f);
    }

    fn compile_expr(&mut self, expr: &IRExpr, f: &mut Function, preallocated: bool) {
        match &expr.node {
            IRExprKind::Integer(n) => {
                f.instruction(&Instruction::I64Const(*n));
            }
            IRExprKind::Float(n) => {
                f.instruction(&Instruction::F64Const(wasm_encoder::Ieee64::from(*n)));
            }
            IRExprKind::Boolean(b) => {
                f.instruction(&Instruction::I32Const(if *b { 1 } else { 0 }));
            }
            IRExprKind::String(_s) => {
                todo!()
            }
            IRExprKind::Null => {
                f.instruction(&Instruction::I64Const(0));
            }
            IRExprKind::Local(index) => {
                f.instruction(&Instruction::LocalGet(*index));
            }
            IRExprKind::Binary {
                left,
                op: BinaryOp::Is,
                right,
            } => {
                if let IRExprKind::Local(index) = &left.node {
                    self.compile_expr(right, f, false);
                    f.instruction(&Instruction::LocalTee(*index));
                } else if let IRExprKind::FieldReference { object, offset } = &left.node {
                    self.compile_expr(left, f, false);
                    f.instruction(&Instruction::LocalTee(0));
                    self.compile_expr(right, f, false);
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                    f.instruction(&Instruction::LocalGet(0));
                } else {
                    self.compile_expr(left, f, false);
                    f.instruction(&Instruction::LocalTee(0));
                    self.compile_expr(right, f, false);
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 1,
                    }));
                    f.instruction(&Instruction::LocalGet(0));
                }
            }
            IRExprKind::Binary { left, op, right } => {
                self.compile_expr(left, f, false);
                self.compile_expr(right, f, false);
                match op {
                    BinaryOp::Plus => {
                        f.instruction(&Instruction::I64Add);
                    }
                    BinaryOp::Minus => {
                        f.instruction(&Instruction::I64Sub);
                    }
                    BinaryOp::Multiply => {
                        f.instruction(&Instruction::I64Mul);
                    }
                    BinaryOp::Divide => {
                        f.instruction(&Instruction::I64DivS);
                    }
                    BinaryOp::BitwiseAnd => {
                        f.instruction(&Instruction::I64And);
                    }
                    BinaryOp::BitwiseOr => {
                        f.instruction(&Instruction::I64Or);
                    }
                    BinaryOp::Eq => {
                        f.instruction(&Instruction::I64Eq);
                    }
                    BinaryOp::Neq => {
                        f.instruction(&Instruction::I64Ne);
                    }
                    BinaryOp::Lt => {
                        f.instruction(&Instruction::I64LtS);
                    }
                    BinaryOp::Gt => {
                        f.instruction(&Instruction::I64GtS);
                    }
                    BinaryOp::Lte => {
                        f.instruction(&Instruction::I64LeS);
                    }
                    BinaryOp::Gte => {
                        f.instruction(&Instruction::I64GeS);
                    }
                    BinaryOp::Sll => {
                        f.instruction(&Instruction::I64Shl);
                    }
                    BinaryOp::Srl => {
                        f.instruction(&Instruction::I64ShrS);
                    }
                    BinaryOp::Xor => {
                        f.instruction(&Instruction::I64Xor);
                    }
                    BinaryOp::And => {
                        f.instruction(&Instruction::I32And);
                    }
                    BinaryOp::Or => {
                        f.instruction(&Instruction::I32Or);
                    }
                    _ => panic!("Unsupported binary operation"),
                }
            }
            IRExprKind::Unary { op, expr } => match op {
                UnaryOp::Minus => {
                    f.instruction(&Instruction::I64Const(0));
                    self.compile_expr(expr, f, false);
                    f.instruction(&Instruction::I64Sub);
                }
                UnaryOp::Not => {
                    self.compile_expr(expr, f, false);
                    f.instruction(&Instruction::I32Eqz);
                }
                UnaryOp::Raise => {}
                UnaryOp::Count => {
                    self.compile_expr(expr, f, false);
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::I32Sub);
                    f.instruction(&Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 1,
                    }));
                    f.instruction(&Instruction::I64ExtendI32U);
                }
            },
            IRExprKind::Call { callee, args } => {
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I64Const(0));
                self.compile_expr(callee, f, false);
                f.instruction(&Instruction::LocalTee(1));
                f.instruction(&Instruction::LocalGet(1));
                f.instruction(&Instruction::I64Const(32));
                f.instruction(&Instruction::I64ShrU);
                f.instruction(&Instruction::I32WrapI64);
                f.instruction(&Instruction::LocalSet(0));
                f.instruction(&Instruction::I32WrapI64);
                for arg in args {
                    self.compile_expr(arg, f, false);
                }
                f.instruction(&Instruction::LocalGet(0));

                f.instruction(&Instruction::CallIndirect {
                    type_index: 5,
                    table_index: 0,
                });
            }
            IRExprKind::New {
                struct_index,
                fields,
            } => {
                if !preallocated {
                    f.instruction(&Instruction::I32Const(*struct_index as i32));
                    f.instruction(&Instruction::Call(3));
                }
                f.instruction(&Instruction::LocalTee(0));
                let mut offset = 0u64;
                for field_expr in fields {
                    self.compile_expr(field_expr, f, false);
                    match field_expr.ty.kind {
                        TypeKind::Struct { .. } => {
                            f.instruction(&Instruction::I32Store(MemArg {
                                offset,
                                align: 2,
                                memory_index: 0,
                            }));
                        }
                        _ => {
                            f.instruction(&Instruction::I64Store(MemArg {
                                offset,
                                align: 3,
                                memory_index: 0,
                            }));
                        }
                    }
                    f.instruction(&Instruction::LocalGet(0));
                    offset += 8;
                }
            }
            IRExprKind::Field { object, offset } => {
                self.compile_expr(object, f, false);
                if matches!(expr.ty.kind, TypeKind::Struct { .. }) {
                    f.instruction(&Instruction::I32Load(MemArg {
                        offset: *offset as u64,
                        align: 2,
                        memory_index: 0,
                    }));
                } else {
                    f.instruction(&Instruction::I64Load(MemArg {
                        offset: *offset as u64,
                        align: 3,
                        memory_index: 0,
                    }));
                }
            }
            IRExprKind::FieldReference { object, offset } => {
                self.compile_expr(object, f, false);
                f.instruction(&Instruction::I32Const(*offset as i32));
                f.instruction(&Instruction::I32Add);
            }
            IRExprKind::IndexReference { list, index } => {
                self.compile_expr(list, f, false);

                self.compile_expr(index, f, false);
                f.instruction(&Instruction::I64Const(8));
                f.instruction(&Instruction::I64Mul);
                f.instruction(&Instruction::I32WrapI64);

                f.instruction(&Instruction::I32Add);
            }

            IRExprKind::List(elements) => {
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::I32Const(elements.len() as i32));
                f.instruction(&Instruction::Call(5));
                f.instruction(&Instruction::LocalTee(0));
                for (_, _) in elements.iter().enumerate() {
                    f.instruction(&Instruction::LocalGet(0));
                }
                for (i, element) in elements.iter().enumerate() {
                    self.compile_expr(element, f, false);
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: (i * 8) as u64,
                        align: 3,
                        memory_index: 1,
                    }));
                }
            }
            IRExprKind::Index { list, index } => {
                self.compile_expr(list, f, false);

                self.compile_expr(index, f, false);
                f.instruction(&Instruction::I64Const(8));
                f.instruction(&Instruction::I64Mul);
                f.instruction(&Instruction::I32WrapI64);

                f.instruction(&Instruction::I32Add);

                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 1,
                }));
            }
            IRExprKind::Match { .. } => todo!(),
            IRExprKind::UnwrapError(_) => todo!(),
            IRExprKind::UnwrapNull(expr) => {
                self.compile_expr(expr, f, false);
                f.instruction(&Instruction::LocalTee(0));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I64Const(0));
                f.instruction(&Instruction::I64Eq);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));
                f.instruction(&Instruction::Unreachable);
                f.instruction(&Instruction::Else);
                f.instruction(&Instruction::LocalGet(0));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 8,
                    align: 3,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::End);
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &IRStmt, f: &mut Function) {
        match stmt {
            IRStmt::Expr(expr) => {
                self.compile_expr(expr, f, false);
                f.instruction(&Instruction::Drop);
            }
            IRStmt::LocalSet { index, value } => {
                self.compile_expr(value, f, false);
                println!("Setting local {} ", index);
                f.instruction(&Instruction::LocalSet(*index));
            }
            IRStmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expr(expr, f, false);
                } else {
                    f.instruction(&Instruction::I64Const(0));
                }
                f.instruction(&Instruction::Return);
            }
            IRStmt::Break => {
                f.instruction(&Instruction::Br(1));
            }
            IRStmt::Continue => {
                f.instruction(&Instruction::Br(0));
            }
            IRStmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.compile_expr(condition, f, false);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                for stmt in then_block {
                    self.compile_stmt(stmt, f);
                }
                if let Some(else_stmts) = else_block {
                    f.instruction(&Instruction::Else);
                    for stmt in else_stmts {
                        self.compile_stmt(stmt, f);
                    }
                }
                f.instruction(&Instruction::End);
            }
            IRStmt::While { condition, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.compile_expr(condition, f, false);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for stmt in body {
                    self.compile_stmt(stmt, f);
                }
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End);
                f.instruction(&Instruction::End);
            }
            IRStmt::For {
                init,
                condition,
                update,
                body,
            } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                self.compile_stmt(init, f);
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.compile_expr(condition, f, false);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for stmt in body {
                    self.compile_stmt(stmt, f);
                }
                self.compile_stmt(update, f);
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End);
                f.instruction(&Instruction::End);
            }
            IRStmt::Print(expr) => {
                self.compile_expr(expr, f, false);
                f.instruction(&Instruction::Call(0));
            }
            IRStmt::Produce(_) => todo!(),
            IRStmt::LocalClosure {
                fn_index,
                captures,
                index,
            } => {
                match &captures.node {
                    IRExprKind::New {
                        struct_index,
                        fields: _,
                    } => {
                        f.instruction(&Instruction::I32Const(*struct_index as i32));
                        f.instruction(&Instruction::Call(3));
                        f.instruction(&Instruction::LocalTee(0));
                    }
                    _ => panic!("Captures must be a local struct allocation"),
                }
                f.instruction(&Instruction::I64ExtendI32U);
                f.instruction(&Instruction::I32Const(*fn_index as i32));
                f.instruction(&Instruction::I64ExtendI32U);
                f.instruction(&Instruction::I64Const(32));
                f.instruction(&Instruction::I64Shl);
                f.instruction(&Instruction::I64Or);
                f.instruction(&Instruction::LocalSet(*index));
                f.instruction(&Instruction::LocalGet(0));
                self.compile_expr(captures, f, true);
            }
        }
    }
}
