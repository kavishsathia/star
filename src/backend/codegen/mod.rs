mod constants;
mod expr;
mod helpers;
mod stmt;

use crate::ast::{IRFunction, IRProgram, Type, TypeKind};
use crate::error::CompilerError;
use wasm_encoder::{
    CodeSection, ConstExpr, ElementSection, Elements, EntityType, ExportSection, FunctionSection,
    ImportSection, Module, RefType, TableSection, TableType, TypeSection, ValType,
};

use constants::{FUNCTION_IMPORTS, IMPORT_COUNT, MEMORY_IMPORTS};
use helpers::type_to_valtype;

pub struct Codegen {
    functions: Vec<IRFunction>,
}

impl Codegen {
    pub fn new() -> Self {
        Codegen { functions: vec![] }
    }

    fn find_type_index(&self, callee_ty: &Type) -> Result<u32, CompilerError> {
        if let TypeKind::Function { params, returns } = &callee_ty.kind {
            for (i, func) in self.functions.iter().enumerate() {
                if func.params == *params && func.returns == **returns {
                    return Ok(IMPORT_COUNT + i as u32);
                }
            }
        }
        Err(CompilerError::Codegen {
            message: "Could not find matching function type for call_indirect".to_string(),
        })
    }

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
            params.extend(func.params.iter().map(type_to_valtype));
            let results: Vec<ValType> = vec![type_to_valtype(&func.returns)];
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
            functions.function((i as u32 + IMPORT_COUNT) as u32);
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
        exports.export("main", wasm_encoder::ExportKind::Func, IMPORT_COUNT);
        module.section(&exports);

        if !program.functions.is_empty() {
            let func_indices: Vec<u32> =
                (IMPORT_COUNT..(IMPORT_COUNT + program.functions.len() as u32)).collect();
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
}
