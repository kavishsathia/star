use crate::ast::{Type, TypeKind};
use wasm_encoder::{Function, Instruction, MemArg, ValType};

use super::constants::{import, mem};

pub fn type_to_valtype(ty: &Type) -> ValType {
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

pub fn emit_gc_retry<P, R, O>(f: &mut Function, prepare: P, retrieve: R, operation: O)
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
pub fn emit_access_cast(f: &mut Function, ty: &TypeKind) {
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
pub fn emit_storage_cast(f: &mut Function, ty: &TypeKind) {
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
pub fn emit_unwrap(f: &mut Function, tag: i64, result_ty: &Type) {
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
        type_to_valtype(result_ty),
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
        emit_access_cast(f, &result_ty.kind);
    }

    f.instruction(&Instruction::End);
}
