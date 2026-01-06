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
use types::TypeChecker;

fn main() {
    let source = r"


        fn main(): integer?! {
            struct Point {
                x: integer
                y: integer
            }

            
            print 2;
            let p: Point = new Point { x: 10, y: 20 };

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

    // Parse
    let mut parser = Parser::new(source);
    let program = parser.parse_program();
    println!("AST: {:?}", program);

    // Index functions
    let mut fn_indexer = FunctionIndexer::new();
    let program = match fn_indexer.index_program(program) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Function indexing error: {}", e);
            return;
        }
    };
    println!("Functions indexed: {} functions", program.function_signatures.len());

    // Index locals
    let mut locals_indexer = LocalsIndexer::new();
    let program = match locals_indexer.index_program(program) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Locals indexing error: {}", e);
            return;
        }
    };
    println!("Locals indexed");

    // Type check
    let mut type_checker = TypeChecker::new();
    let program = match type_checker.check_program(program) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Type error: {}", e.message);
            return;
        }
    };
    println!("Type checked, found {} struct types", program.struct_types.len());

    // Codegen
    let mut codegen = Codegen::new();
    let wasm_bytes = codegen.compile(&program);

    std::fs::write("output.wasm", &wasm_bytes).unwrap();
    println!("Wrote {} bytes to output.wasm", wasm_bytes.len());
    println!("\nRun with: cargo run --bin run");
}

