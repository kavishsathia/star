use std::{fmt::Binary, mem::discriminant};

use wasm_encoder::{
    CodeSection, ExportSection, Function, FunctionSection,
    ImportSection, Instruction, Module, TypeSection, ValType, EntityType,
};
use crate::ast::{Expr, BinaryOp, UnaryOp, Statement};

pub struct Codegen {}

impl Codegen {
    pub fn new() -> Self {
        Codegen {}
    }

    pub fn compile(&mut self, stmts: &[Statement]) -> Vec<u8> {
        let mut module = Module::new();


        let mut types = TypeSection::new();
        types.ty().function(vec![ValType::I64], vec![]); 
        types.ty().function(vec![], vec![]);
        module.section(&types);

        // Import section: import print_i64 from host
        let mut imports = ImportSection::new();
        imports.import("env", "print_i64", EntityType::Function(0)); // function index 0
        module.section(&imports);

        let mut functions = FunctionSection::new();
        functions.function(1); // main uses type 1
        module.section(&functions);

        let mut exports = ExportSection::new();
        exports.export("main", wasm_encoder::ExportKind::Func, 1);
        module.section(&exports);

        let mut codes = CodeSection::new();
        let mut f = Function::new(vec![(5, ValType::I64)]);

        for stmt in stmts {
            self.compile_stmt(stmt, &mut f);
        }

        f.instruction(&Instruction::End);
        codes.function(&f);
        module.section(&codes);

        module.finish()
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
            _ => panic!("Unsupported expression type"),
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
                todo!()
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
            Statement::Function { name, params, return_type, body, local_types } => {
                todo!()
            }
            Statement::Struct { name, fields } => {
                todo!()
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
