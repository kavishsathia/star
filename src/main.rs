mod lexer;
mod ast;
mod parser;
mod types;
mod codegen;
mod locals;
mod function;

use parser::Parser;
use codegen::Codegen;
use locals::LocalsIndexer;
use function::FunctionIndexer;

fn main() {
    let source = r"
        

        fn main(): integer?! {
            fn add(a: integer, b: integer): integer {
                return a + b;
            }
            let x: integer = 0;
            let y: integer = 1;
            let n: integer = 10;

            while n > 0 {
                let tmp: integer = x;
                x = y;
                y = add(tmp, y);
                n = n - 1;
            }

            for let i: integer = 0; i < 10; i = i + 1; {
                print i;
            }

            print x;
            return 0;
        }
    ";

    println!("Compiling: {}", source);

    let mut parser = Parser::new(source);
    let stmts = parser.parse_program();

    println!("AST: {:?}", stmts);

    let mut fn_indexer = FunctionIndexer::new();
    let program = match fn_indexer.index_program(stmts) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Function indexing error: {}", e);
            return;
        }
    };
    println!("Functions indexed: {} functions", fn_indexer.function_signatures.len());

    let mut locals_indexer = LocalsIndexer::new();
    for stmt in &program.statements {
        if let Err(e) = locals_indexer.index_stmt(stmt) {
            eprintln!("Locals indexing error: {}", e);
            return;
        }
    }
    println!("Locals indexed");

    let mut codegen = Codegen::new();
    let wasm_bytes = codegen.compile(program);

    std::fs::write("output.wasm", &wasm_bytes).unwrap();
    println!("Wrote {} bytes to output.wasm", wasm_bytes.len());
    println!("\nRun with: cargo run --bin run");
}

