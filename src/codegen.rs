use crate::ast::{BinaryOp, Type, TypeKind, UnaryOp};
use crate::error::CompilerError;
use crate::ir::{IRExpr, IRExprKind, IRFunction, IRProgram, IRStmt, IRStruct};
use wasm_encoder::{
    CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemArg, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

pub struct Codegen {
    functions: Vec<IRFunction>,
}

impl Codegen {
    fn type_to_valtype(ty: &Type) -> ValType {
        if ty.nullable || ty.errorable {
            return ValType::I32;
        }
        match &ty.kind {
            TypeKind::String => ValType::I32,
            TypeKind::Function { .. } => ValType::I64,
            TypeKind::List { .. } => ValType::I32,
            TypeKind::Struct { .. } => ValType::I32,
            TypeKind::Boolean => ValType::I32,
            TypeKind::Float => ValType::F64,
            _ => ValType::I64,
        }
    }
}

impl Codegen {
    pub fn new() -> Self {
        Codegen { functions: vec![] }
    }

    fn emit_gc_retry<P, R, O>(f: &mut Function, prepare: P, retrieve: R, operation: O)
    where
        P: Fn(&mut Function),
        R: Fn(&mut Function),
        O: Fn(&mut Function),
    {
        prepare(f);

        retrieve(f);
        operation(f);

        f.instruction(&Instruction::LocalTee(0));
        f.instruction(&Instruction::I32Eqz);
        f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));

        f.instruction(&Instruction::Call(17)); // gc
        retrieve(f);
        operation(f);
        f.instruction(&Instruction::LocalSet(0));

        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(0));
    }

    fn find_type_index(&self, callee_ty: &Type) -> Result<u32, CompilerError> {
        if let TypeKind::Function { params, returns } = &callee_ty.kind {
            for (i, func) in self.functions.iter().enumerate() {
                if func.params == *params && func.returns == **returns {
                    return Ok(Self::IMPORT_COUNT + i as u32);
                }
            }
        }
        Err(CompilerError::Codegen {
            message: "Could not find matching function type for call_indirect".to_string(),
        })
    }

    const IMPORT_COUNT: u32 = 18;

    pub fn compile(&mut self, program: &IRProgram) -> Result<Vec<u8>, CompilerError> {
        self.functions = program.functions.clone();
        let mut module = Module::new();

        let mut types = TypeSection::new();
        types.ty().function(vec![ValType::I32], vec![]); // 0: print
        types.ty().function(vec![], vec![]); // 1: init
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32, ValType::I32], vec![]); // 2: register
        types.ty().function(vec![ValType::I32], vec![ValType::I32]); // 3: falloc
        types.ty().function(vec![], vec![]); // 4: dinit
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32], vec![ValType::I32]); // 5: dalloc
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32], vec![ValType::I32]); // 6: dconcat
        types.ty().function(
            vec![ValType::I32, ValType::I32, ValType::I32],
            vec![ValType::I32],
        ); // 7: dslice
        types
            .ty()
            .function(vec![ValType::I64, ValType::I32], vec![ValType::I32]); // 8: din_u64
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32], vec![ValType::I32]); // 9: deq
        types.ty().function(vec![ValType::I64], vec![ValType::I32]); // 10: ditoa
        types.ty().function(vec![ValType::I32], vec![ValType::I32]); // 11: dbtoa
        types.ty().function(vec![ValType::F64], vec![ValType::I32]); // 12: dftoa
        types.ty().function(vec![], vec![]); // 13: sinit
        types.ty().function(vec![ValType::I32], vec![]); // 14: spush
        types.ty().function(vec![], vec![]); // 15: spop
        types
            .ty()
            .function(vec![ValType::I32, ValType::I32, ValType::I32], vec![]); // 16: sset
        types.ty().function(vec![], vec![]); // 17: gc

        for func in &program.functions {
            let mut params: Vec<ValType> = vec![ValType::I32, ValType::I64, ValType::I32];
            params.extend(func.params.iter().map(Self::type_to_valtype));
            let results: Vec<ValType> = vec![Self::type_to_valtype(&func.returns)];
            types.ty().function(params, results);
        }
        module.section(&types);

        let mut imports = ImportSection::new();
        imports.import("env", "print", EntityType::Function(0));
        imports.import("alloc", "init", EntityType::Function(1));
        imports.import("alloc", "register", EntityType::Function(2));
        imports.import("alloc", "falloc", EntityType::Function(3));
        imports.import("dalloc", "dinit", EntityType::Function(4));
        imports.import("dalloc", "dalloc", EntityType::Function(5));
        imports.import("dalloc", "dconcat", EntityType::Function(6));
        imports.import("dalloc", "dslice", EntityType::Function(7));
        imports.import("dalloc", "din_u64", EntityType::Function(8));
        imports.import("dalloc", "deq", EntityType::Function(9));
        imports.import("dalloc", "ditoa", EntityType::Function(10));
        imports.import("dalloc", "dbtoa", EntityType::Function(11));
        imports.import("dalloc", "dftoa", EntityType::Function(12));
        imports.import("shadow", "init", EntityType::Function(13));
        imports.import("shadow", "push", EntityType::Function(14));
        imports.import("shadow", "pop", EntityType::Function(15));
        imports.import("shadow", "set", EntityType::Function(16));
        imports.import("shadow", "gc", EntityType::Function(17));
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
        imports.import(
            "shadow",
            "memory",
            EntityType::Memory(wasm_encoder::MemoryType {
                minimum: 1,
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
            self.compile_function(func, &mut codes, program)?;
        }

        module.section(&codes);

        Ok(module.finish())
    }

    fn compile_function(
        &mut self,
        func: &IRFunction,
        codes: &mut CodeSection,
        program: &IRProgram,
    ) -> Result<(), CompilerError> {
        let mut locals: Vec<(u32, ValType)> = vec![];
        locals.extend(func.locals.iter().map(|t| (1, Self::type_to_valtype(t))));
        let mut f = Function::new(locals);

        if func.name == "main" {
            f.instruction(&Instruction::Call(1));
            f.instruction(&Instruction::Call(4));
            f.instruction(&Instruction::Call(13));
            for ir_struct in &program.structs {
                f.instruction(&Instruction::I32Const(ir_struct.size as i32));
                f.instruction(&Instruction::I32Const(ir_struct.struct_count as i32));
                f.instruction(&Instruction::I32Const(ir_struct.list_count as i32));
                f.instruction(&Instruction::Call(2));
            }
        }

        let frame_size = 1 + func.params.len() + func.locals.len();
        f.instruction(&Instruction::I32Const(frame_size as i32));
        f.instruction(&Instruction::Call(14));

        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Call(16));

        for (i, param_ty) in func.params.iter().enumerate() {
            let local_index = 3 + i as u32;
            let shadow_slot = 1 + i as i32;
            match &param_ty.kind {
                TypeKind::Struct { .. } => {
                    f.instruction(&Instruction::LocalGet(local_index));
                    f.instruction(&Instruction::I32Const(shadow_slot));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::Call(16));
                }
                TypeKind::List { .. } | TypeKind::String => {
                    f.instruction(&Instruction::LocalGet(local_index));
                    f.instruction(&Instruction::I32Const(shadow_slot));
                    f.instruction(&Instruction::I32Const(2));
                    f.instruction(&Instruction::Call(16));
                }
                _ => {}
            }
        }

        for stmt in &func.body {
            self.compile_stmt(stmt, &mut f)?;
        }

        f.instruction(&Instruction::Call(15));
        f.instruction(&Instruction::End);
        codes.function(&f);
        Ok(())
    }

    fn compile_expr(
        &mut self,
        expr: &IRExpr,
        f: &mut Function,
        preallocated: bool,
    ) -> Result<(), CompilerError> {
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
            IRExprKind::String(s) => {
                let len = s.len() as i32;
                Self::emit_gc_retry(
                    f,
                    |f| {
                        // prepare: store params to scratchpad at memory 2, bytes 4-11
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(1)); // type = 1
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(len));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: 2,
                        }));
                    },
                    |f| {
                        // retrieve: load params from scratchpad
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: 2,
                        }));
                    },
                    |f| {
                        // operation: call dalloc
                        f.instruction(&Instruction::Call(5));
                    },
                );

                for _ in 0..s.len() {
                    f.instruction(&Instruction::LocalGet(0));
                }

                for (i, byte) in s.bytes().enumerate() {
                    f.instruction(&Instruction::I32Const(byte as i32));
                    f.instruction(&Instruction::I32Store(MemArg {
                        offset: (i * 8) as u64,
                        align: 2,
                        memory_index: 1,
                    }));
                }
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
                    self.compile_expr(right, f, false)?;
                    f.instruction(&Instruction::LocalTee(*index));
                    match &right.ty.kind {
                        TypeKind::Struct { .. } => {
                            f.instruction(&Instruction::I32Const((*index - 2) as i32));
                            f.instruction(&Instruction::I32Const(1));
                            f.instruction(&Instruction::Call(16));
                            f.instruction(&Instruction::LocalGet(*index));
                        }
                        TypeKind::List { .. } | TypeKind::String => {
                            f.instruction(&Instruction::I32Const((*index - 2) as i32));
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::Call(16));
                            f.instruction(&Instruction::LocalGet(*index));
                        }
                        _ => {}
                    }
                } else if let IRExprKind::FieldReference { object, offset } = &left.node {
                    self.compile_expr(left, f, false)?;
                    f.instruction(&Instruction::LocalTee(0));
                    self.compile_expr(right, f, false)?;
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 0,
                    }));
                    f.instruction(&Instruction::LocalGet(0));
                } else {
                    self.compile_expr(left, f, false)?;
                    f.instruction(&Instruction::LocalTee(0));
                    self.compile_expr(right, f, false)?;
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: 1,
                    }));
                    f.instruction(&Instruction::LocalGet(0));
                }
            }
            IRExprKind::Binary { left, op, right } => {
                self.compile_expr(left, f, false)?;
                self.compile_expr(right, f, false)?;
                match op {
                    BinaryOp::Plus => {
                        if left.ty.kind == TypeKind::Integer {
                            f.instruction(&Instruction::I64Add);
                            return Ok(());
                        } else if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Add);
                            return Ok(());
                        } else {
                            Self::emit_gc_retry(
                                f,
                                |f| {
                                    // stack: [left, right] -> store both
                                    f.instruction(&Instruction::LocalSet(0)); // right -> local0
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::LocalGet(0));
                                    f.instruction(&Instruction::I32Store(MemArg {
                                        offset: 8,
                                        align: 2,
                                        memory_index: 2,
                                    }));
                                    f.instruction(&Instruction::LocalSet(0)); // left -> local0
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::LocalGet(0));
                                    f.instruction(&Instruction::I32Store(MemArg {
                                        offset: 4,
                                        align: 2,
                                        memory_index: 2,
                                    }));
                                },
                                |f| {
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::I32Load(MemArg {
                                        offset: 4,
                                        align: 2,
                                        memory_index: 2,
                                    }));
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::I32Load(MemArg {
                                        offset: 8,
                                        align: 2,
                                        memory_index: 2,
                                    }));
                                },
                                |f| {
                                    f.instruction(&Instruction::Call(6));
                                },
                            );
                            return Ok(());
                        }
                    }
                    BinaryOp::Minus => {
                        if left.ty.kind == TypeKind::Integer {
                            f.instruction(&Instruction::I64Sub);
                            return Ok(());
                        } else if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Sub);
                            return Ok(());
                        } else {
                            return Err(CompilerError::Codegen {
                                message: "Cannot subtract non-numeric types".to_string(),
                            });
                        }
                    }
                    BinaryOp::Multiply => {
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Mul);
                        } else {
                            f.instruction(&Instruction::I64Mul);
                        }
                    }
                    BinaryOp::Divide => {
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Div);
                        } else {
                            f.instruction(&Instruction::I64DivS);
                        }
                    }
                    BinaryOp::BitwiseAnd => {
                        f.instruction(&Instruction::I64And);
                    }
                    BinaryOp::BitwiseOr => {
                        f.instruction(&Instruction::I64Or);
                    }
                    BinaryOp::Eq => {
                        if left.ty.kind == TypeKind::String
                            || matches!(left.ty.kind, TypeKind::List { .. })
                        {
                            f.instruction(&Instruction::Call(9));
                            return Ok(());
                        }
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Eq);
                        } else {
                            f.instruction(&Instruction::I64Eq);
                        }
                    }
                    BinaryOp::Neq => {
                        if left.ty.kind == TypeKind::String
                            || matches!(left.ty.kind, TypeKind::List { .. })
                        {
                            f.instruction(&Instruction::Call(9));
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Eqz);
                            return Ok(());
                        }
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Ne);
                        } else {
                            f.instruction(&Instruction::I64Ne);
                        }
                    }
                    BinaryOp::Lt => {
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Lt);
                        } else {
                            f.instruction(&Instruction::I64LtS);
                        }
                    }
                    BinaryOp::Gt => {
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Gt);
                        } else {
                            f.instruction(&Instruction::I64GtS);
                        }
                    }
                    BinaryOp::Lte => {
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Le);
                        } else {
                            f.instruction(&Instruction::I64LeS);
                        }
                    }
                    BinaryOp::Gte => {
                        if left.ty.kind == TypeKind::Float {
                            f.instruction(&Instruction::F64Ge);
                        } else {
                            f.instruction(&Instruction::I64GeS);
                        }
                    }
                    BinaryOp::Modulo => {
                        f.instruction(&Instruction::I64RemS);
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
                    BinaryOp::In => {
                        f.instruction(&Instruction::Call(8));
                    }
                    _ => {
                        return Err(CompilerError::Codegen {
                            message: format!("Unsupported binary operation: {:?}", op),
                        })
                    }
                }
            }
            IRExprKind::Unary { op, expr } => match op {
                UnaryOp::Minus => {
                    if expr.ty.kind == TypeKind::Float {
                        self.compile_expr(expr, f, false)?;
                        f.instruction(&Instruction::F64Neg);
                    } else {
                        f.instruction(&Instruction::I64Const(0));
                        self.compile_expr(expr, f, false)?;
                        f.instruction(&Instruction::I64Sub);
                    }
                }
                UnaryOp::Not => {
                    self.compile_expr(expr, f, false)?;
                    f.instruction(&Instruction::I32Eqz);
                }
                UnaryOp::Count => {
                    self.compile_expr(expr, f, false)?;
                    f.instruction(&Instruction::I32Const(4));
                    f.instruction(&Instruction::I32Sub);
                    f.instruction(&Instruction::I32Load(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 1,
                    }));
                    f.instruction(&Instruction::I64ExtendI32U);
                }
                UnaryOp::Stringify => match expr.ty.kind {
                    TypeKind::Integer => {
                        self.compile_expr(expr, f, false)?;
                        Self::emit_gc_retry(
                            f,
                            |f| {
                                f.instruction(&Instruction::LocalSet(1)); // i64 needs local1
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::LocalGet(1));
                                f.instruction(&Instruction::I64Store(MemArg {
                                    offset: 4,
                                    align: 3,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I64Load(MemArg {
                                    offset: 4,
                                    align: 3,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(10));
                            },
                        );
                    }
                    TypeKind::String => {
                        self.compile_expr(expr, f, false)?;
                    }
                    TypeKind::Boolean => {
                        self.compile_expr(expr, f, false)?;
                        Self::emit_gc_retry(
                            f,
                            |f| {
                                f.instruction(&Instruction::LocalSet(0));
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::LocalGet(0));
                                f.instruction(&Instruction::I32Store(MemArg {
                                    offset: 4,
                                    align: 2,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I32Load(MemArg {
                                    offset: 4,
                                    align: 2,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(11));
                            },
                        );
                    }
                    TypeKind::Float => {
                        self.compile_expr(expr, f, false)?;
                        Self::emit_gc_retry(
                            f,
                            |f| {
                                f.instruction(&Instruction::LocalSet(1)); // f64 needs local1
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::LocalGet(1));
                                f.instruction(&Instruction::F64Store(MemArg {
                                    offset: 4,
                                    align: 3,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::F64Load(MemArg {
                                    offset: 4,
                                    align: 3,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(12));
                            },
                        );
                    }
                    _ => {
                        return Err(CompilerError::Codegen {
                            message: format!("Cannot stringify type {:?}", expr.ty),
                        })
                    }
                },
            },
            IRExprKind::Call { callee, args } => {
                let type_index = self.find_type_index(&callee.ty)?;
                f.instruction(&Instruction::I32Const(0));
                f.instruction(&Instruction::I64Const(0));
                self.compile_expr(callee, f, false)?;
                f.instruction(&Instruction::LocalTee(1));
                f.instruction(&Instruction::LocalGet(1));
                f.instruction(&Instruction::I64Const(32));
                f.instruction(&Instruction::I64ShrU);
                f.instruction(&Instruction::I32WrapI64);
                f.instruction(&Instruction::LocalSet(0));
                f.instruction(&Instruction::I32WrapI64);
                for arg in args {
                    self.compile_expr(arg, f, false)?;
                }
                f.instruction(&Instruction::LocalGet(0));

                f.instruction(&Instruction::CallIndirect {
                    type_index,
                    table_index: 0,
                });
            }
            IRExprKind::New {
                struct_index,
                fields,
            } => {
                if !preallocated {
                    let idx = *struct_index as i32;
                    Self::emit_gc_retry(
                        f,
                        |f| {
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Const(idx));
                            f.instruction(&Instruction::I32Store(MemArg {
                                offset: 4,
                                align: 2,
                                memory_index: 2,
                            }));
                        },
                        |f| {
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Load(MemArg {
                                offset: 4,
                                align: 2,
                                memory_index: 2,
                            }));
                        },
                        |f| {
                            f.instruction(&Instruction::Call(3));
                        },
                    );
                }
                f.instruction(&Instruction::LocalTee(0));

                for _ in fields {
                    f.instruction(&Instruction::LocalGet(0));
                }

                let mut offset = 0u64;
                for field_expr in fields {
                    self.compile_expr(field_expr, f, false)?;
                    // Convert to i64 for uniform 8-byte storage
                    match field_expr.ty.kind {
                        TypeKind::Struct { .. }
                        | TypeKind::List { .. }
                        | TypeKind::String
                        | TypeKind::Boolean => {
                            f.instruction(&Instruction::I64ExtendI32U);
                        }
                        TypeKind::Float => {
                            f.instruction(&Instruction::I64ReinterpretF64);
                        }
                        _ => {
                            // Integer, Function - already i64
                        }
                    }
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset,
                        align: 3,
                        memory_index: 0,
                    }));
                    offset += 8;
                }
            }
            IRExprKind::Field { object, offset } => {
                self.compile_expr(object, f, false)?;
                // Load as i64, then convert back to original type
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: *offset as u64,
                    align: 3,
                    memory_index: 0,
                }));
                match expr.ty.kind {
                    TypeKind::Struct { .. }
                    | TypeKind::List { .. }
                    | TypeKind::String
                    | TypeKind::Boolean => {
                        f.instruction(&Instruction::I32WrapI64);
                    }
                    TypeKind::Float => {
                        f.instruction(&Instruction::F64ReinterpretI64);
                    }
                    _ => {
                        // Integer, Function - already i64
                    }
                }
            }
            IRExprKind::FieldReference { object, offset } => {
                self.compile_expr(object, f, false)?;
                f.instruction(&Instruction::I32Const(*offset as i32));
                f.instruction(&Instruction::I32Add);
            }
            IRExprKind::IndexReference { list, index } => {
                self.compile_expr(list, f, false)?;

                self.compile_expr(index, f, false)?;
                f.instruction(&Instruction::I64Const(8));
                f.instruction(&Instruction::I64Mul);
                f.instruction(&Instruction::I32WrapI64);

                f.instruction(&Instruction::I32Add);
            }

            IRExprKind::Slice { expr, start, end } => {
                self.compile_expr(expr, f, false)?;
                self.compile_expr(start, f, false)?;
                f.instruction(&Instruction::I32WrapI64);
                self.compile_expr(end, f, false)?;
                f.instruction(&Instruction::I32WrapI64);
                Self::emit_gc_retry(
                    f,
                    |f| {
                        // stack: [ptr, start, end] -> store all 3
                        f.instruction(&Instruction::LocalSet(0)); // end -> local0
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(0));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 12,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::LocalSet(0)); // start -> local0
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(0));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::LocalSet(0)); // ptr -> local0
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(0));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 2,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 12,
                            align: 2,
                            memory_index: 2,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::Call(7));
                    },
                );
            }

            IRExprKind::List(elements) => {
                let len = elements.len() as i32;
                Self::emit_gc_retry(
                    f,
                    |f| {
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(len));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: 2,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: 2,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: 2,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::Call(5));
                    },
                );
                f.instruction(&Instruction::LocalTee(0));
                for (_, _) in elements.iter().enumerate() {
                    f.instruction(&Instruction::LocalGet(0));
                }
                for (i, element) in elements.iter().enumerate() {
                    self.compile_expr(element, f, false)?;
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: (i * 8) as u64,
                        align: 3,
                        memory_index: 1,
                    }));
                }
            }
            IRExprKind::Index { list, index } => {
                self.compile_expr(list, f, false)?;

                self.compile_expr(index, f, false)?;
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
            IRExprKind::UnwrapError(inside) => {
                self.compile_expr(inside, f, false)?;
                f.instruction(&Instruction::LocalTee(0));
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: 0,
                    align: 3,
                    memory_index: 0,
                }));
                f.instruction(&Instruction::I64Const(1));
                f.instruction(&Instruction::I64Eq);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
                    ValType::I64,
                )));
                f.instruction(&Instruction::Unreachable);
                f.instruction(&Instruction::Else);
                f.instruction(&Instruction::LocalGet(0));
                if !expr.ty.nullable && !expr.ty.errorable {
                    f.instruction(&Instruction::I64Load(MemArg {
                        offset: 8,
                        align: 3,
                        memory_index: 0,
                    }));
                } else {
                }
                f.instruction(&Instruction::End);
            }
            IRExprKind::UnwrapNull(inside) => {
                self.compile_expr(inside, f, false)?;
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
                if !expr.ty.nullable && !expr.ty.errorable {
                    f.instruction(&Instruction::I64Load(MemArg {
                        offset: 8,
                        align: 3,
                        memory_index: 0,
                    }));
                } else {
                }
                f.instruction(&Instruction::End);
            }
        }
        Ok(())
    }

    fn compile_stmt(&mut self, stmt: &IRStmt, f: &mut Function) -> Result<(), CompilerError> {
        match stmt {
            IRStmt::Expr(expr) => {
                self.compile_expr(expr, f, false)?;
                f.instruction(&Instruction::Drop);
            }
            IRStmt::LocalSet { index, value } => {
                self.compile_expr(value, f, false)?;
                f.instruction(&Instruction::LocalTee(*index));
                match value.ty.kind {
                    TypeKind::Struct { .. } => {
                        f.instruction(&Instruction::I32Const((*index - 2) as i32));
                        f.instruction(&Instruction::I32Const(1));
                        f.instruction(&Instruction::Call(16));
                    }
                    TypeKind::List { .. } | TypeKind::String => {
                        f.instruction(&Instruction::I32Const((*index - 2) as i32));
                        f.instruction(&Instruction::I32Const(2));
                        f.instruction(&Instruction::Call(16));
                    }
                    _ => {
                        f.instruction(&Instruction::Drop);
                    }
                }
            }
            IRStmt::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expr(expr, f, false)?;
                } else {
                    f.instruction(&Instruction::I64Const(0));
                }
                f.instruction(&Instruction::Call(15));
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
                self.compile_expr(condition, f, false)?;
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                for stmt in then_block {
                    self.compile_stmt(stmt, f)?;
                }
                if let Some(else_stmts) = else_block {
                    f.instruction(&Instruction::Else);
                    for stmt in else_stmts {
                        self.compile_stmt(stmt, f)?;
                    }
                }
                f.instruction(&Instruction::End);
            }
            IRStmt::While { condition, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.compile_expr(condition, f, false)?;
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for stmt in body {
                    self.compile_stmt(stmt, f)?;
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
                self.compile_stmt(init, f)?;
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.compile_expr(condition, f, false)?;
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for stmt in body {
                    self.compile_stmt(stmt, f)?;
                }
                self.compile_stmt(update, f)?;
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End);
                f.instruction(&Instruction::End);
            }
            IRStmt::Print(expr) => {
                self.compile_expr(expr, f, false)?;
                f.instruction(&Instruction::Call(0));
            }
            IRStmt::Produce(_) => todo!(),
            IRStmt::Raise(expr) => {
                self.compile_expr(expr, f, false)?;
                f.instruction(&Instruction::Call(15));
                f.instruction(&Instruction::Return);
            }
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
                        let idx = *struct_index as i32;
                        Self::emit_gc_retry(
                            f,
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I32Const(idx));
                                f.instruction(&Instruction::I32Store(MemArg {
                                    offset: 4,
                                    align: 2,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I32Load(MemArg {
                                    offset: 4,
                                    align: 2,
                                    memory_index: 2,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(3));
                            },
                        );
                        f.instruction(&Instruction::LocalTee(0));
                    }
                    _ => {
                        return Err(CompilerError::Codegen {
                            message: "Captures must be a local struct allocation".to_string(),
                        })
                    }
                }
                f.instruction(&Instruction::I64ExtendI32U);
                f.instruction(&Instruction::I32Const(*fn_index as i32));
                f.instruction(&Instruction::I64ExtendI32U);
                f.instruction(&Instruction::I64Const(32));
                f.instruction(&Instruction::I64Shl);
                f.instruction(&Instruction::I64Or);
                f.instruction(&Instruction::LocalSet(*index));
                f.instruction(&Instruction::LocalGet(0));
                f.instruction(&Instruction::I32Const((*index - 2) as i32));
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::Call(16));
                f.instruction(&Instruction::LocalGet(0));
                self.compile_expr(captures, f, true)?;
            }
        }
        Ok(())
    }
}
