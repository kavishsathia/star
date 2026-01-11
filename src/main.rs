mod lexer;
mod ast;
mod aast;
mod parser;
mod ir;
mod tast;
mod types;
mod locals;
mod fast;
mod flatten;
mod irgen;
mod codegen;

use std::time::Instant;
use parser::Parser;
use types::TypeChecker;
use locals::LocalsIndexer;
use flatten::Flattener;
use irgen::IRGenerator;
use codegen::Codegen;

fn main() {
    let source = r#"
        struct Node {
            value: integer,
            next: Node?,
        }

        fn main(): integer {
            let node: Node = new Node {
                value: 10,
                next: null,
            };

            node.value = 20;
            return node.value;
        }
    "#;

    println!("Parsing...\n");

    let parse_start = Instant::now();
    let mut parser = Parser::new(source);
    let program = parser.parse_program();
    let parse_duration = parse_start.elapsed();
    println!("AST: {:#?}\n", program);
    println!("Parsing took: {:?}\n", parse_duration);

    println!("Type checking...\n");

    let typecheck_start = Instant::now();
    let mut type_checker = TypeChecker::new();
    let result = type_checker.check_program(&program);
    let typecheck_duration = typecheck_start.elapsed();
    match result {
        Ok(typed_program) => {
            println!("TypedAST: {:#?}\n", typed_program);
            println!("Type checking took: {:?}\n", typecheck_duration);

            println!("Analyzing...\n");

            let analyze_start = Instant::now();
            let mut indexer = LocalsIndexer::new();
            let analyzed_program = indexer.analyze_program(&typed_program);
            let analyze_duration = analyze_start.elapsed();
            println!("AnalyzedAST: {:#?}\n", analyzed_program);
            println!("Analysis took: {:?}\n", analyze_duration);

            println!("Flattening...\n");

            let flatten_start = Instant::now();
            let mut flattener = Flattener::new();
            let flattened_program = flattener.flatten_program(&analyzed_program);
            let flatten_duration = flatten_start.elapsed();
            println!("FlattenedAST: {:#?}\n", flattened_program);
            println!("Flattening took: {:?}\n", flatten_duration);

            println!("Generating IR...\n");

            let irgen_start = Instant::now();
            let mut ir_generator = IRGenerator::new();
            let ir_program = ir_generator.generate(&flattened_program);
            let irgen_duration = irgen_start.elapsed();
            println!("IR: {:#?}\n", ir_program);
            println!("IR generation took: {:?}\n", irgen_duration);

            println!("Compiling to WASM...\n");

            let codegen_start = Instant::now();
            let mut codegen = Codegen::new();
            let wasm_bytes = codegen.compile(&ir_program);
            let codegen_duration = codegen_start.elapsed();
            println!("WASM bytes: {} bytes", wasm_bytes.len());
            println!("Codegen took: {:?}\n", codegen_duration);

            std::fs::write("output.wasm", &wasm_bytes).expect("Failed to write output.wasm");
            println!("Written to output.wasm");
        }
        Err(e) => {
            println!("Type error: {}", e.message);
            println!("Type checking took: {:?}", typecheck_duration);
        }
    }
}
