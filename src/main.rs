mod lexer;
mod ast;
mod parser;
mod types;
mod codegen;

use parser::Parser;
use codegen::Codegen;

fn main() {
    let source = "if false { 1 + 2; } else if false { 5 + 6; } else { 3 + 4; }";

    println!("Compiling: {}", source);

    let mut parser = Parser::new(source);
    let stmt = parser.parse_statement();

    println!("AST: {:?}", stmt);

    let mut codegen = Codegen::new();
    let wasm_bytes = codegen.compile(&[stmt]);

    std::fs::write("output.wasm", &wasm_bytes).unwrap();
    println!("Wrote {} bytes to output.wasm", wasm_bytes.len());
    println!("\nRun with: wasmtime output.wasm --invoke main");
}

