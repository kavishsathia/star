use wasm_encoder::ValType;

/// Declarative import function definitions
/// The index of each entry becomes the function index used in Call instructions
pub struct ImportDef {
    pub module: &'static str,
    pub name: &'static str,
    pub params: &'static [ValType],
    pub results: &'static [ValType],
}

pub const FUNCTION_IMPORTS: &[ImportDef] = &[
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
pub mod import {
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
pub struct MemoryImportDef {
    pub module: &'static str,
    pub name: &'static str,
    pub min_pages: u64,
}

pub const MEMORY_IMPORTS: &[MemoryImportDef] = &[
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
pub mod mem {
    pub const ALLOC: u32 = 0; // Fixed allocator memory (structs)
    pub const DALLOC: u32 = 1; // Dynamic allocator memory (lists, strings)
    pub const SHADOW: u32 = 2; // Shadow stack memory (GC roots + scratchpad)
}

pub const IMPORT_COUNT: u32 = FUNCTION_IMPORTS.len() as u32;
