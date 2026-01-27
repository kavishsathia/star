use star::compile;
use std::process;
use std::time::Instant;

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
