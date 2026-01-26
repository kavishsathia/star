mod aast;
mod ast;
mod codegen;
mod error;
mod fast;
mod flatten;
mod ir;
mod irgen;
mod lexer;
mod locals;
mod parser;
mod tast;
mod types;
mod wrap;

use codegen::Codegen;
use flatten::Flattener;
use irgen::IRGenerator;
use locals::LocalsIndexer;
use parser::Parser;
use std::process;
use std::time::Instant;
use types::TypeChecker;
use wrap::Wrapper;

fn main() {
    let source = r#"
error Hello;
fn main(): integer {
    fn maybe(): integer?! {
        // raise new Hello { message: "An error occurred" };
        // return 42;
        return null;
    }

    maybe()??;
return 0;
}
    "#;

    println!("Parsing...\n");

    let parse_start = Instant::now();
    let mut parser = Parser::new(source);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    };
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
            let analyzed_program = match indexer.analyze_program(&typed_program) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            let analyze_duration = analyze_start.elapsed();
            println!("AnalyzedAST: {:#?}\n", analyzed_program);
            println!("Analysis took: {:?}\n", analyze_duration);

            println!("Flattening...\n");

            let flatten_start = Instant::now();
            let mut flattener = Flattener::new();
            let flattened_program = flattener.flatten_program(&analyzed_program);
            let flatten_duration = flatten_start.elapsed();
            println!("Flattening took: {:?}\n", flatten_duration);

            println!("Wrapping...\n");

            let wrap_start = Instant::now();
            let mut wrapper = Wrapper::new();
            let wrapped_program = match wrapper.wrap_program(flattened_program) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            let wrap_duration = wrap_start.elapsed();
            println!("Wrapping took: {:?}\n", wrap_duration);

            println!("Generating IR...\n");

            let irgen_start = Instant::now();
            let mut ir_generator = IRGenerator::new();
            let ir_program = match ir_generator.generate(&wrapped_program) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            let irgen_duration = irgen_start.elapsed();
            println!("IR: {:#?}\n", ir_program);
            println!("IR generation took: {:?}\n", irgen_duration);

            println!("Compiling to WASM...\n");

            let codegen_start = Instant::now();
            let mut codegen = Codegen::new();
            let wasm_bytes = match codegen.compile(&ir_program) {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("Error: {}", e);
                    process::exit(1);
                }
            };
            let codegen_duration = codegen_start.elapsed();
            println!("WASM bytes: {} bytes", wasm_bytes.len());
            println!("Codegen took: {:?}\n", codegen_duration);

            std::fs::write("output.wasm", &wasm_bytes).expect("Failed to write output.wasm");
            println!("Written to output.wasm");
        }
        Err(e) => {
            eprintln!("Type error: {}", e.message);
            eprintln!("Type checking took: {:?}", typecheck_duration);
            process::exit(1);
        }
    }
}
