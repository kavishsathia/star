mod lexer;
mod ast;
mod parser;
mod types;
mod codegen;

use parser::Parser;
use codegen::Codegen;

fn main() {
    let source = r"
       print (true and false);
    
    ";

    println!("Compiling: {}", source);

    let mut parser = Parser::new(source);
    let stmt = parser.parse_statement();

    println!("AST: {:?}", stmt);

    let mut codegen = Codegen::new();
    let wasm_bytes = codegen.compile(&[stmt]);

    std::fs::write("output.wasm", &wasm_bytes).unwrap();
    println!("Wrote {} bytes to output.wasm", wasm_bytes.len());
    println!("\nRun with: cargo run --bin run");
}

