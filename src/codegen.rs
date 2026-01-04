use wasm_encoder::{
    CodeSection, ExportSection, Function, FunctionSection,
    Instruction, Module, TypeSection,
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
        types.ty().function(vec![], vec![]); // () -> () for statements
        module.section(&types);

        let mut functions = FunctionSection::new();
        functions.function(0);
        module.section(&functions);

        let mut exports = ExportSection::new();
        exports.export("main", wasm_encoder::ExportKind::Func, 0);
        module.section(&exports);

        let mut codes = CodeSection::new();
        let mut f = Function::new(vec![]);

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
            Expr::Binary { left, op, right } => {
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
                    _ => panic!("Unsupported binary operation"),
                }
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
                todo!()
            }
            Statement::Let { name, value, type_annotation } => {
                todo!()
            }
            Statement::Const { name, value, type_annotation } => {
                todo!()
            }
            Statement::Return(expr) => {
                todo!()
            }
            Statement::Break => {
                todo!()
            }
            Statement::Continue => {
                todo!()
            }
            Statement::For { initializer, condition, increment, body } => {
                todo!()
            }
            Statement::Function { name, params, return_type, body } => {
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
        }
    }
}
