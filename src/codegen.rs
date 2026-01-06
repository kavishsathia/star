use std::{fmt::Binary, mem::discriminant};

use wasm_encoder::{
    CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportSection, Function, FunctionSection, ImportSection, Instruction, MemArg, Module, RefType, TableSection, TableType, TypeSection, ValType
};
use crate::ast::{BinaryOp, Expr, Program, Statement, Type, TypeKind, UnaryOp};

pub struct Codegen {}

impl Codegen {
    fn type_to_valtype(ty: &Type) -> ValType {
        match &ty.kind {
            TypeKind::Function { .. } => ValType::I32,
            TypeKind::Primitive(name) if name == "Point" => ValType::I32,
            _ => ValType::I64,
        }
    }
}

impl Codegen {
    pub fn new() -> Self {
        Codegen {}
    }

    const IMPORT_COUNT: u32 = 4;

    pub fn compile(&mut self, program: &Program) -> Vec<u8> {
        let stmts = &program.statements;
        let fn_sigs = &program.function_signatures;
        let mut module = Module::new();

        let mut types = TypeSection::new();
        // Type 0: (i64) -> () for print_i64
        types.ty().function(vec![ValType::I64], vec![]);
        // Type 1: () -> () for init
        types.ty().function(vec![], vec![]);
        // Type 2: (i32) -> () for register
        types.ty().function(vec![ValType::I32], vec![]);
        // Type 3: (i32) -> i32 for falloc
        types.ty().function(vec![ValType::I32], vec![ValType::I32]);
        // Type 4+: Star function types
        for (_name, param_types, return_type) in fn_sigs {
            let params: Vec<ValType> = param_types.iter().map(Self::type_to_valtype).collect();
            let results: Vec<ValType> = vec![Self::type_to_valtype(return_type)];
            types.ty().function(params, results);
        }
        module.section(&types);

        let mut imports = ImportSection::new();
        imports.import("env", "print_i64", EntityType::Function(0));
        imports.import("alloc", "init", EntityType::Function(1));
        imports.import("alloc", "register", EntityType::Function(2));
        imports.import("alloc", "falloc", EntityType::Function(3));
        imports.import("alloc", "memory", EntityType::Memory(wasm_encoder::MemoryType {
            minimum: 1,
            maximum: None,
            memory64: false,
            shared: false,
            page_size_log2: None,
        }));
        module.section(&imports);

        let mut functions = FunctionSection::new();
        for (i, _) in fn_sigs.iter().enumerate() {
            functions.function((i as u32 + Self::IMPORT_COUNT) as u32);
        }
        module.section(&functions);

        if !fn_sigs.is_empty() {
            let mut tables = TableSection::new();
            tables.table(TableType {
                element_type: RefType::FUNCREF,
                minimum: fn_sigs.len() as u64,
                maximum: Some(fn_sigs.len() as u64),
                table64: false,
                shared: false,
            });
            module.section(&tables);
        }

        let mut exports = ExportSection::new();
        exports.export("main", wasm_encoder::ExportKind::Func, Self::IMPORT_COUNT);
        module.section(&exports);

        if !fn_sigs.is_empty() {
            let func_indices: Vec<u32> = (Self::IMPORT_COUNT..(Self::IMPORT_COUNT + fn_sigs.len() as u32)).collect();
            let mut elements = ElementSection::new();
            elements.active(
                Some(0),
                &ConstExpr::i32_const(0),
                Elements::Functions(std::borrow::Cow::Borrowed(&func_indices)),
            );
            module.section(&elements);
        }

        let mut codes = CodeSection::new();

        for stmt in stmts {
            if let Statement::Function { .. } = stmt {
                self.compile_function(stmt, &mut codes, program);
            }
        }

        module.section(&codes);

        module.finish()
    }

    fn compile_function(&mut self, stmt: &Statement, codes: &mut CodeSection, program: &Program) {
        if let Statement::Function { name   , body, local_types, function_index, local_index, .. } = stmt {
            let local_types_vec = local_types.borrow();
            let locals: Vec<(u32, ValType)> = local_types_vec
                .iter()
                .map(|t| (1, Self::type_to_valtype(t)))
                .collect();
            let mut f = Function::new(locals);

            if name == "main" {
                f.instruction(&Instruction::Call(1));
                for struct_type in &program.struct_types {
                    let (_name, size) = struct_type;
                    f.instruction(&Instruction::I32Const(*size as i32));
                    f.instruction(&Instruction::Call(2)); 
                }
            }

            for body_stmt in body {
                self.compile_stmt(body_stmt, &mut f);
            }

            f.instruction(&Instruction::End);
            codes.function(&f);

            for body_stmt in body {
                if let Statement::Function { .. } = body_stmt {
                    self.compile_function(body_stmt, codes, program);
                }
            }
        } else {
            panic!("compile_function called with non-function statement");
        }
    }

    fn compile_expr(&mut self, expr: &Expr, f: &mut Function) {
        match expr {
            Expr::Integer(n) => {
                f.instruction(&Instruction::I64Const(*n));
            }
            Expr::Float(n) => {
                f.instruction(&Instruction::F64Const(wasm_encoder::Ieee64::from(*n)));
            }
            Expr::Boolean(b) => {
                f.instruction(&Instruction::I32Const(if *b { 1 } else { 0 }));
            }
            Expr::Binary { left, op: BinaryOp::Is, right } =>  {
                self.compile_expr(right, f);
                if let Expr::Identifier { local_index, .. } = &**left {
                    let index = local_index.get().expect("Local index not set for identifier");
                    f.instruction(&Instruction::LocalTee(index));
                } else {
                    panic!("Left side of 'is' must be an identifier");
                }
            }
            Expr::Binary { left, op, right }  => {
                self.compile_expr(left, f);
                self.compile_expr(right, f);
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
            Expr::Identifier { name, local_index } => {
                let index = local_index.get().expect("Local index not set for identifier");
                f.instruction(&Instruction::LocalGet(index));
            }
            Expr::Unary { op, expr } => {
                match op {
                    UnaryOp::Minus => {
                        f.instruction(&Instruction::I64Const(0));
                        self.compile_expr(expr, f);
                        f.instruction(&Instruction::I64Sub);
                    }
                    UnaryOp::Not => {
                        self.compile_expr(expr, f);
                        f.instruction(&Instruction::I32Eqz);
                    }
                    UnaryOp::Raise => {
                        // Placeholder for error raising
                    }
                }
            }
            Expr::Call { callee, args } => {
                for arg in args {
                    self.compile_expr(arg, f);
                }
                self.compile_expr(callee, f);
                f.instruction(&Instruction::CallIndirect {
                    type_index: 5, // Placeholder type index
                    table_index: 0,
                });
            }
            Expr::Init { name: _, fields, type_index } => {
                type_index.get().map(|idx| {
                    f.instruction(&Instruction::I32Const(idx as i32));
                    f.instruction(&Instruction::Call(3));

                    f.instruction(&Instruction::LocalTee(0));

                    let mut offset = 0;
                    for (field_name, field_expr) in fields {
                        self.compile_expr(field_expr, f);
                        f.instruction(&Instruction::I64Store(MemArg { offset, align: 3, memory_index: 0 }));
                        f.instruction(&Instruction::LocalGet(0));
                        offset += 8;
                    }
            });
            }
            Expr::MemberAccess { object, field } => {
                self.compile_expr(object, f);
                f.instruction(&Instruction::I64Load(MemArg { offset: 0, align: 3, memory_index: 0 }));
            }
            _ => {
                panic!("Unsupported expression type in codegen");
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Statement, f: &mut Function) {
        match stmt {
            Statement::Expr(expr) => {
                self.compile_expr(expr, f);
                f.instruction(&Instruction::Drop);
            }
            Statement::If { condition, consequent, alternate } => {
                self.compile_expr(condition, f);
                f.instruction(&Instruction::If(wasm_encoder::BlockType::Empty));
                for stmt in consequent {    
                    self.compile_stmt(stmt, f);
                }
                if let Some(alternate) = alternate {
                    f.instruction(&Instruction::Else);
                    for stmt in alternate {
                        self.compile_stmt(stmt, f);
                    }
                }
                f.instruction(&Instruction::End);
            }
            Statement::While { condition, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.compile_expr(condition, f);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for stmt in body {
                    self.compile_stmt(stmt, f);
                }
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End);
                f.instruction(&Instruction::End);
            }
            Statement::Let { name, value, type_annotation, local_index } => {
                if let Some(expr) = value {
                    self.compile_expr(expr, f);
                } else {
                    f.instruction(&Instruction::I64Const(0));
                }
                let index = local_index.get().expect("Local index not set for variable");
                f.instruction(&Instruction::LocalSet(index));
            }
            Statement::Const { name, value, type_annotation, local_index } => {
                self.compile_expr(value, f);
                let index = local_index.get().expect("Local index not set for constant");
                f.instruction(&Instruction::LocalSet(index));
            }
            Statement::Return(expr) => {
                if let Some(expr) = expr {
                    self.compile_expr(expr, f);
                } else {
                    f.instruction(&Instruction::I64Const(0));
                }
                f.instruction(&Instruction::Return);
            }
            Statement::Break => {
                f.instruction(&Instruction::Br(1));
            }
            Statement::Continue => {
                f.instruction(&Instruction::Br(0));
            }
            Statement::For { initializer, condition, increment, body } => {
                f.instruction(&Instruction::Block(wasm_encoder::BlockType::Empty));
                self.compile_stmt(initializer, f);
                f.instruction(&Instruction::Loop(wasm_encoder::BlockType::Empty));
                self.compile_expr(condition, f);
                f.instruction(&Instruction::I32Eqz);
                f.instruction(&Instruction::BrIf(1));
                for stmt in body {
                    self.compile_stmt(stmt, f);
                }
                self.compile_stmt(increment, f);
                f.instruction(&Instruction::Br(0));
                f.instruction(&Instruction::End);
                f.instruction(&Instruction::End);
            }
            Statement::Function { name, params, return_type, body, local_types, function_index, local_index } => {
                f.instruction(&Instruction::I32Const(function_index.get().expect("Function index not set") as i32));
                f.instruction(&Instruction::LocalSet(local_index.get().expect("Local index not set for function")));
            }
            Statement::Struct { name, fields } => {
                ()
            }
            Statement::Error { name } => {
                todo!()
            }
            Statement::Match { expr, arms } => {
                todo!()
            }
            Statement::Print(expr) => {
                self.compile_expr(expr, f);
                f.instruction(&Instruction::Call(0)); // call print_i64 (function index 0)
            }
        }
    }
}
