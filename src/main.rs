mod lexer;
mod ast;
mod parser;
mod types;
mod codegen;

use parser::Parser;
use codegen::Codegen;

fn main() {
    let source = "1 + 2 * 3";

    println!("Compiling: {}", source);

    let mut parser = Parser::new(source);
    let expr = parser.parse_expression(0);

    println!("AST: {:?}", expr);

    let mut codegen = Codegen::new();
    let wasm_bytes = codegen.compile(&expr);

    std::fs::write("output.wasm", &wasm_bytes).unwrap();
    println!("Wrote {} bytes to output.wasm", wasm_bytes.len());
    println!("\nRun with: wasmtime output.wasm --invoke main");
}

