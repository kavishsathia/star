use crate::ast::{IRExprKind, IRFunction, IRProgram, IRStmt, TypeKind};
use crate::error::CompilerError;
use wasm_encoder::{CodeSection, Function, Instruction, MemArg};

use super::constants::{import, mem};
use super::helpers::{emit_gc_retry, type_to_valtype};
use super::Codegen;

impl Codegen {
    pub(super) fn compile_function(
        &mut self,
        func: &IRFunction,
        codes: &mut CodeSection,
        program: &IRProgram,
    ) -> Result<(), CompilerError> {
        let mut locals: Vec<(u32, wasm_encoder::ValType)> = vec![];
        locals.extend(func.locals.iter().map(|t| (1, type_to_valtype(t))));
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

    pub(super) fn compile_stmt(
        &mut self,
        stmt: &IRStmt,
        f: &mut Function,
    ) -> Result<(), CompilerError> {
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
                        emit_gc_retry(
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
                // BUG
                f.instruction(&Instruction::I32Const(1));
                f.instruction(&Instruction::Call(import::SHADOW_SET));
                f.instruction(&Instruction::LocalGet(0));
                self.compile_expr(captures, f, true)?;
            }
        }
        Ok(())
    }
}
