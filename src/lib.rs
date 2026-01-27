pub mod ast;
pub mod error;
mod frontend;
mod analysis;
mod transforms;
mod backend;

use backend::Codegen;
use error::CompilerError;
use transforms::{Flattener, Wrapper};
use backend::IRGenerator;
use analysis::LocalsIndexer;
use frontend::Parser;
use analysis::TypeChecker;

/// Compiles Star source code to WASM bytes.
/// Returns Ok(wasm_bytes) on success, Err(CompilerError) on failure.
pub fn compile(source: &str) -> Result<Vec<u8>, CompilerError> {
    let mut parser = Parser::new(source);
    let program = parser.parse_program()?;

    let mut type_checker = TypeChecker::new();
    let typed_program = type_checker
        .check_program(&program)
        .map_err(|e| CompilerError::Type { message: e.message })?;

    let mut indexer = LocalsIndexer::new();
    let analyzed_program = indexer.analyze_program(&typed_program)?;

    let mut flattener = Flattener::new();
    let flattened_program = flattener.flatten_program(&analyzed_program);

    let mut wrapper = Wrapper::new();
    let wrapped_program = wrapper.wrap_program(flattened_program)?;

    let mut ir_generator = IRGenerator::new();
    let ir_program = ir_generator.generate(&wrapped_program)?;

    let mut codegen = Codegen::new();
    codegen.compile(&ir_program)
}

// WASM exports for browser
#[cfg(target_arch = "wasm32")]
mod wasm_exports {
    use super::compile;

    static mut RESULT_BUFFER: Vec<u8> = Vec::new();
    static mut ERROR_BUFFER: String = String::new();

    /// Allocate memory for passing strings from JS
    #[no_mangle]
    pub extern "C" fn wasm_alloc(len: usize) -> *mut u8 {
        let mut buf = Vec::with_capacity(len);
        let ptr = buf.as_mut_ptr();
        std::mem::forget(buf);
        ptr
    }

    /// Compile source code, returns 1 on success, 0 on error
    #[no_mangle]
    pub extern "C" fn wasm_compile(ptr: *const u8, len: usize) -> i32 {
        let source = unsafe {
            let slice = std::slice::from_raw_parts(ptr, len);
            match std::str::from_utf8(slice) {
                Ok(s) => s,
                Err(_) => {
                    ERROR_BUFFER = "Invalid UTF-8 input".to_string();
                    return 0;
                }
            }
        };

        match compile(source) {
            Ok(bytes) => unsafe {
                RESULT_BUFFER = bytes;
                1
            },
            Err(e) => unsafe {
                ERROR_BUFFER = e.to_string();
                0
            },
        }
    }

    /// Get pointer to compiled WASM bytes
    #[no_mangle]
    pub extern "C" fn wasm_result_ptr() -> *const u8 {
        unsafe { RESULT_BUFFER.as_ptr() }
    }

    /// Get length of compiled WASM bytes
    #[no_mangle]
    pub extern "C" fn wasm_result_len() -> usize {
        unsafe { RESULT_BUFFER.len() }
    }

    /// Get pointer to error message
    #[no_mangle]
    pub extern "C" fn wasm_error_ptr() -> *const u8 {
        unsafe { ERROR_BUFFER.as_ptr() }
    }

    /// Get length of error message
    #[no_mangle]
    pub extern "C" fn wasm_error_len() -> usize {
        unsafe { ERROR_BUFFER.len() }
    }
}
