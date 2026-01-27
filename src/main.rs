use star::compile;
use std::process;
use std::time::Instant;

fn main() {
    let source = r#"
fn main(): integer {
    fn add(x: integer, y: integer): integer {
        return x + y;
    }

    fn mult(x: integer, y: integer): integer {
        return x * y;
    }
    
    print $(10 + add(add(1, mult(3, 4)), 2));
    return 0;
}


    "#;

    println!("Compiling...\n");

    let start = Instant::now();
    match compile(source) {
        Ok(wasm_bytes) => {
            let duration = start.elapsed();
            println!("WASM bytes: {} bytes", wasm_bytes.len());
            println!("Compilation took: {:?}\n", duration);

            std::fs::write("output.wasm", &wasm_bytes).expect("Failed to write output.wasm");
            println!("Written to output.wasm");
        }
        Err(e) => {
            let duration = start.elapsed();
            eprintln!("Error: {}", e);
            eprintln!("Compilation took: {:?}", duration);
            process::exit(1);
        }
    }
}
