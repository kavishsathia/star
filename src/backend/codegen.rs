use crate::ast::{BinaryOp, Type, TypeKind, UnaryOp};
use crate::ast::{IRExpr, IRExprKind, IRFunction, IRProgram, IRStmt, IRStruct};
use crate::error::CompilerError;
use wasm_encoder::{
    CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportSection, Function,
    FunctionSection, ImportSection, Instruction, MemArg, Module, RefType, TableSection, TableType,
    TypeSection, ValType,
};

/// Declarative import function definitions
/// The index of each entry becomes the function index used in Call instructions
struct ImportDef {
    module: &'static str,
    name: &'static str,
    params: &'static [ValType],
    results: &'static [ValType],
}

const FUNCTION_IMPORTS: &[ImportDef] = &[
    ImportDef {
        module: "env",
        name: "print",
        params: &[ValType::I32],
        results: &[],
    },
    ImportDef {
        module: "alloc",
        name: "init",
        params: &[],
        results: &[],
    },
    ImportDef {
        module: "alloc",
        name: "register",
        params: &[ValType::I32, ValType::I32, ValType::I32],
        results: &[],
    },
    ImportDef {
        module: "alloc",
        name: "falloc",
        params: &[ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "dinit",
        params: &[],
        results: &[],
    },
    ImportDef {
        module: "dalloc",
        name: "dalloc",
        params: &[ValType::I32, ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "dconcat",
        params: &[ValType::I32, ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "dslice",
        params: &[ValType::I32, ValType::I32, ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "din_u64",
        params: &[ValType::I64, ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "deq",
        params: &[ValType::I32, ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "ditoa",
        params: &[ValType::I64],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "dbtoa",
        params: &[ValType::I32],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "dalloc",
        name: "dftoa",
        params: &[ValType::F64],
        results: &[ValType::I32],
    },
    ImportDef {
        module: "shadow",
        name: "init",
        params: &[],
        results: &[],
    },
    ImportDef {
        module: "shadow",
        name: "push",
        params: &[ValType::I32],
        results: &[],
    },
    ImportDef {
        module: "shadow",
        name: "pop",
        params: &[],
        results: &[],
    },
    ImportDef {
        module: "shadow",
        name: "set",
        params: &[ValType::I32, ValType::I32, ValType::I32],
        results: &[],
    },
    ImportDef {
        module: "shadow",
        name: "gc",
        params: &[],
        results: &[],
    },
];

/// Import function indices - derived from FUNCTION_IMPORTS array position
mod import {
    pub const PRINT: u32 = 0;
    pub const ALLOC_INIT: u32 = 1;
    pub const ALLOC_REGISTER: u32 = 2;
    pub const FALLOC: u32 = 3;
    pub const DINIT: u32 = 4;
    pub const DALLOC: u32 = 5;
    pub const DCONCAT: u32 = 6;
    pub const DSLICE: u32 = 7;
    pub const DIN_U64: u32 = 8;
    pub const DEQ: u32 = 9;
    pub const DITOA: u32 = 10;
    pub const DBTOA: u32 = 11;
    pub const DFTOA: u32 = 12;
    pub const SHADOW_INIT: u32 = 13;
    pub const SHADOW_PUSH: u32 = 14;
    pub const SHADOW_POP: u32 = 15;
    pub const SHADOW_SET: u32 = 16;
    pub const GC: u32 = 17;
}

/// Memory import definitions
struct MemoryImportDef {
    module: &'static str,
    name: &'static str,
    min_pages: u64,
}

const MEMORY_IMPORTS: &[MemoryImportDef] = &[
    MemoryImportDef {
        module: "alloc",
        name: "memory",
        min_pages: 1,
    },
    MemoryImportDef {
        module: "dalloc",
        name: "memory",
        min_pages: 16,
    },
    MemoryImportDef {
        module: "shadow",
        name: "memory",
        min_pages: 1,
    },
];

/// Memory indices for the three memory spaces
mod mem {
    pub const ALLOC: u32 = 0; // Fixed allocator memory (structs)
    pub const DALLOC: u32 = 1; // Dynamic allocator memory (lists, strings)
    pub const SHADOW: u32 = 2; // Shadow stack memory (GC roots + scratchpad)
}

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

        f.instruction(&Instruction::Call(import::GC));
        retrieve(f);
        operation(f);
        f.instruction(&Instruction::LocalSet(0));

        f.instruction(&Instruction::End);

        f.instruction(&Instruction::LocalGet(0));
    }

    /// Emit instructions to convert a value from i64 storage format to its actual runtime type.
    /// Values are stored as i64 in memory, but need conversion for pointer types and floats.
    fn emit_access_cast(f: &mut Function, ty: &TypeKind) {
        match ty {
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

    /// Emit instructions to convert a value from its runtime type to i64 storage format.
    /// Inverse of emit_access_cast.
    fn emit_storage_cast(f: &mut Function, ty: &TypeKind) {
        match ty {
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
    }

    /// Emit code to unwrap a nullable or errorable value.
    /// `tag` is 0 for null-check, 1 for error-check.
    /// `result_ty` is the type after unwrapping.
    fn emit_unwrap(f: &mut Function, tag: i64, result_ty: &Type) {
        let fully_unwrapped = !result_ty.nullable && !result_ty.errorable;

        f.instruction(&Instruction::LocalTee(0));
        f.instruction(&Instruction::I64Load(MemArg {
            offset: 0,
            align: 3,
            memory_index: mem::ALLOC,
        }));
        f.instruction(&Instruction::I64Const(tag));
        f.instruction(&Instruction::I64Eq);

        f.instruction(&Instruction::If(wasm_encoder::BlockType::Result(
            Self::type_to_valtype(result_ty),
        )));

        f.instruction(&Instruction::Unreachable);
        f.instruction(&Instruction::Else);
        f.instruction(&Instruction::LocalGet(0));

        if fully_unwrapped {
            f.instruction(&Instruction::I64Load(MemArg {
                offset: 8,
                align: 3,
                memory_index: mem::ALLOC,
            }));
            Self::emit_access_cast(f, &result_ty.kind);
        }

        f.instruction(&Instruction::End);
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

    const IMPORT_COUNT: u32 = FUNCTION_IMPORTS.len() as u32;

    /// Build the type section from declarative imports + program functions
    fn build_type_section(&self, program: &IRProgram) -> TypeSection {
        let mut types = TypeSection::new();

        // Add types for imported functions
        for def in FUNCTION_IMPORTS {
            types
                .ty()
                .function(def.params.to_vec(), def.results.to_vec());
        }

        // Add types for program functions
        for func in &program.functions {
            let mut params: Vec<ValType> = vec![ValType::I32, ValType::I64, ValType::I32];
            params.extend(func.params.iter().map(Self::type_to_valtype));
            let results: Vec<ValType> = vec![Self::type_to_valtype(&func.returns)];
            types.ty().function(params, results);
        }

        types
    }

    /// Build the import section from declarative imports
    fn build_import_section(&self) -> ImportSection {
        let mut imports = ImportSection::new();

        // Add function imports
        for (i, def) in FUNCTION_IMPORTS.iter().enumerate() {
            imports.import(def.module, def.name, EntityType::Function(i as u32));
        }

        // Add memory imports
        for mem_def in MEMORY_IMPORTS {
            imports.import(
                mem_def.module,
                mem_def.name,
                EntityType::Memory(wasm_encoder::MemoryType {
                    minimum: mem_def.min_pages,
                    maximum: None,
                    memory64: false,
                    shared: false,
                    page_size_log2: None,
                }),
            );
        }

        imports
    }

    pub fn compile(&mut self, program: &IRProgram) -> Result<Vec<u8>, CompilerError> {
        self.functions = program.functions.clone();
        let mut module = Module::new();

        module.section(&self.build_type_section(program));
        module.section(&self.build_import_section());

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
            f.instruction(&Instruction::Call(import::ALLOC_INIT));
            f.instruction(&Instruction::Call(import::DINIT));
            f.instruction(&Instruction::Call(import::SHADOW_INIT));
            for ir_struct in &program.structs {
                f.instruction(&Instruction::I32Const(ir_struct.size as i32));
                f.instruction(&Instruction::I32Const(ir_struct.struct_count as i32));
                f.instruction(&Instruction::I32Const(ir_struct.list_count as i32));
                f.instruction(&Instruction::Call(import::ALLOC_REGISTER));
            }
        }

        let frame_size = 1 + func.params.len() + func.locals.len();
        f.instruction(&Instruction::I32Const(frame_size as i32));
        f.instruction(&Instruction::Call(import::SHADOW_PUSH));

        f.instruction(&Instruction::LocalGet(2));
        f.instruction(&Instruction::I32Const(0));
        f.instruction(&Instruction::I32Const(1));
        f.instruction(&Instruction::Call(import::SHADOW_SET));

        for (i, param_ty) in func.params.iter().enumerate() {
            let local_index = 3 + i as u32;
            let shadow_slot = 1 + i as i32;
            match &param_ty.kind {
                TypeKind::Struct { .. } => {
                    f.instruction(&Instruction::LocalGet(local_index));
                    f.instruction(&Instruction::I32Const(shadow_slot));
                    f.instruction(&Instruction::I32Const(1));
                    f.instruction(&Instruction::Call(import::SHADOW_SET));
                }
                TypeKind::List { .. } | TypeKind::String => {
                    f.instruction(&Instruction::LocalGet(local_index));
                    f.instruction(&Instruction::I32Const(shadow_slot));
                    f.instruction(&Instruction::I32Const(2));
                    f.instruction(&Instruction::Call(import::SHADOW_SET));
                }
                _ => {}
            }
        }

        for stmt in &func.body {
            self.compile_stmt(stmt, &mut f)?;
        }

        f.instruction(&Instruction::Call(import::SHADOW_POP));
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
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(len));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                    },
                    |f| {
                        // retrieve: load params from scratchpad
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                    },
                    |f| {
                        // operation: call dalloc
                        f.instruction(&Instruction::Call(import::DALLOC));
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
                        memory_index: mem::DALLOC,
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
                            f.instruction(&Instruction::Call(import::SHADOW_SET));
                            f.instruction(&Instruction::LocalGet(*index));
                        }
                        TypeKind::List { .. } | TypeKind::String => {
                            f.instruction(&Instruction::I32Const((*index - 2) as i32));
                            f.instruction(&Instruction::I32Const(2));
                            f.instruction(&Instruction::Call(import::SHADOW_SET));
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
                        memory_index: mem::ALLOC,
                    }));
                    f.instruction(&Instruction::LocalGet(0));
                } else {
                    self.compile_expr(left, f, false)?;
                    f.instruction(&Instruction::LocalTee(0));
                    self.compile_expr(right, f, false)?;
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset: 0,
                        align: 3,
                        memory_index: mem::DALLOC,
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
                                        memory_index: mem::SHADOW,
                                    }));
                                    f.instruction(&Instruction::LocalSet(0)); // left -> local0
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::LocalGet(0));
                                    f.instruction(&Instruction::I32Store(MemArg {
                                        offset: 4,
                                        align: 2,
                                        memory_index: mem::SHADOW,
                                    }));
                                },
                                |f| {
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::I32Load(MemArg {
                                        offset: 4,
                                        align: 2,
                                        memory_index: mem::SHADOW,
                                    }));
                                    f.instruction(&Instruction::I32Const(0));
                                    f.instruction(&Instruction::I32Load(MemArg {
                                        offset: 8,
                                        align: 2,
                                        memory_index: mem::SHADOW,
                                    }));
                                },
                                |f| {
                                    f.instruction(&Instruction::Call(import::DCONCAT));
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
                            f.instruction(&Instruction::Call(import::DEQ));
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
                            f.instruction(&Instruction::Call(import::DEQ));
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
                        f.instruction(&Instruction::Call(import::DIN_U64));
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
                        memory_index: mem::DALLOC,
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
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I64Load(MemArg {
                                    offset: 4,
                                    align: 3,
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(import::DITOA));
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
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I32Load(MemArg {
                                    offset: 4,
                                    align: 2,
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(import::DBTOA));
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
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::F64Load(MemArg {
                                    offset: 4,
                                    align: 3,
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(import::DFTOA));
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
                                memory_index: mem::SHADOW,
                            }));
                        },
                        |f| {
                            f.instruction(&Instruction::I32Const(0));
                            f.instruction(&Instruction::I32Load(MemArg {
                                offset: 4,
                                align: 2,
                                memory_index: mem::SHADOW,
                            }));
                        },
                        |f| {
                            f.instruction(&Instruction::Call(import::FALLOC));
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
                    Self::emit_storage_cast(f, &field_expr.ty.kind);
                    f.instruction(&Instruction::I64Store(MemArg {
                        offset,
                        align: 3,
                        memory_index: mem::ALLOC,
                    }));
                    offset += 8;
                }
            }
            IRExprKind::Field { object, offset } => {
                self.compile_expr(object, f, false)?;
                f.instruction(&Instruction::I64Load(MemArg {
                    offset: *offset as u64,
                    align: 3,
                    memory_index: mem::ALLOC,
                }));
                Self::emit_access_cast(f, &expr.ty.kind);
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
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::LocalSet(0)); // start -> local0
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(0));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::LocalSet(0)); // ptr -> local0
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::LocalGet(0));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 12,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::Call(import::DSLICE));
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
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Const(len));
                        f.instruction(&Instruction::I32Store(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 4,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                        f.instruction(&Instruction::I32Const(0));
                        f.instruction(&Instruction::I32Load(MemArg {
                            offset: 8,
                            align: 2,
                            memory_index: mem::SHADOW,
                        }));
                    },
                    |f| {
                        f.instruction(&Instruction::Call(import::DALLOC));
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
                        memory_index: mem::DALLOC,
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
                    memory_index: mem::DALLOC,
                }));
            }
            IRExprKind::Match { .. } => todo!(),
            IRExprKind::UnwrapError(inside) => {
                self.compile_expr(inside, f, false)?;
                Self::emit_unwrap(f, 1, &expr.ty);
            }
            IRExprKind::UnwrapNull(inside) => {
                self.compile_expr(inside, f, false)?;
                Self::emit_unwrap(f, 0, &expr.ty);
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
                        f.instruction(&Instruction::Call(import::SHADOW_SET));
                    }
                    TypeKind::List { .. } | TypeKind::String => {
                        f.instruction(&Instruction::I32Const((*index - 2) as i32));
                        f.instruction(&Instruction::I32Const(2));
                        f.instruction(&Instruction::Call(import::SHADOW_SET));
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
                f.instruction(&Instruction::Call(import::SHADOW_POP));
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
                f.instruction(&Instruction::Call(import::PRINT));
            }
            IRStmt::Produce(_) => todo!(),
            IRStmt::Raise(expr) => {
                self.compile_expr(expr, f, false)?;
                f.instruction(&Instruction::Call(import::SHADOW_POP));
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
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::I32Const(0));
                                f.instruction(&Instruction::I32Load(MemArg {
                                    offset: 4,
                                    align: 2,
                                    memory_index: mem::SHADOW,
                                }));
                            },
                            |f| {
                                f.instruction(&Instruction::Call(import::FALLOC));
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
                f.instruction(&Instruction::Call(import::SHADOW_SET));
                f.instruction(&Instruction::LocalGet(0));
                self.compile_expr(captures, f, true)?;
            }
        }
        Ok(())
    }
}
