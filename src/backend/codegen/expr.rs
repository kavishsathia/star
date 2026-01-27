use crate::ast::{BinaryOp, TypeKind, UnaryOp};
use crate::ast::{IRExpr, IRExprKind};
use crate::error::CompilerError;
use wasm_encoder::{Function, Instruction, MemArg};

use super::constants::{import, mem};
use super::helpers::{emit_access_cast, emit_gc_retry, emit_storage_cast, emit_unwrap};
use super::Codegen;

impl Codegen {
    pub(super) fn compile_expr(
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
                emit_gc_retry(
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
                            emit_gc_retry(
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
                        emit_gc_retry(
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
                        emit_gc_retry(
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
                        emit_gc_retry(
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
                f.instruction(&Instruction::I32WrapI64);
                f.instruction(&Instruction::LocalGet(1));
                f.instruction(&Instruction::I64Const(32));
                f.instruction(&Instruction::I64ShrU);
                f.instruction(&Instruction::I32WrapI64);
                for arg in args {
                    self.compile_expr(arg, f, false)?;
                    emit_storage_cast(f, &arg.ty.kind);
                    f.instruction(&Instruction::LocalSet(1));
                    f.instruction(&Instruction::LocalSet(0));
                    f.instruction(&Instruction::LocalGet(1));
                    emit_access_cast(f, &arg.ty.kind);
                    f.instruction(&Instruction::LocalGet(0));
                }

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
                }
                f.instruction(&Instruction::LocalTee(0));

                for _ in fields {
                    f.instruction(&Instruction::LocalGet(0));
                }

                let mut offset = 0u64;
                for field_expr in fields {
                    self.compile_expr(field_expr, f, false)?;
                    emit_storage_cast(f, &field_expr.ty.kind);
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
                emit_access_cast(f, &expr.ty.kind);
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
                emit_gc_retry(
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
                emit_gc_retry(
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
                emit_unwrap(f, 1, &expr.ty);
            }
            IRExprKind::UnwrapNull(inside) => {
                self.compile_expr(inside, f, false)?;
                emit_unwrap(f, 0, &expr.ty);
            }
        }
        Ok(())
    }
}
