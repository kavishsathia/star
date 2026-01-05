mod lexer;
mod ast;
mod parser;
mod types;
mod codegen;
mod locals;

use parser::Parser;
use codegen::Codegen;
use locals::LocalsIndexer;

fn main() {
    let source = r"
        let x: integer = 0;
        let y: integer = 1;
        let n: integer = 10;

        while n > 0 {
            let tmp: integer = x;
            x = y;
            y = tmp + y;
            n = n - 1;
        }

        print x;
    ";

    println!("Compiling: {}", source);

    let mut parser = Parser::new(source);
    let stmts = parser.parse_program();

    println!("AST: {:?}", stmts);

    let mut indexer = LocalsIndexer::new();
    for stmt in &stmts {
        if let Err(e) = indexer.index_stmt(stmt) {
            eprintln!("Indexing error: {}", e);
            return;
        }
    }
    println!("Locals indexed: {} variables", indexer.next_index);

    let mut codegen = Codegen::new();
    let wasm_bytes = codegen.compile(&stmts);

    std::fs::write("output.wasm", &wasm_bytes).unwrap();
    println!("Wrote {} bytes to output.wasm", wasm_bytes.len());
    println!("\nRun with: cargo run --bin run");
}

