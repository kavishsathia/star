use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use wasmtime::*;

fn run_program(source: &str) -> Result<Vec<String>, String> {
    let wasm_bytes = star::compile(source).map_err(|e| e.to_string())?;

    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    let manifest_dir = env!("CARGO_MANIFEST_DIR");

    let alloc_bytes = fs::read(format!(
        "{}/alloc/target/wasm32-unknown-unknown/release/alloc.wasm",
        manifest_dir
    ))
    .map_err(|e| format!("Failed to read alloc.wasm: {}", e))?;
    let alloc_module = Module::new(&engine, &alloc_bytes).map_err(|e| e.to_string())?;
    let alloc_instance = linker
        .instantiate(&mut store, &alloc_module)
        .map_err(|e| e.to_string())?;
    linker
        .instance(&mut store, "alloc", alloc_instance)
        .map_err(|e| e.to_string())?;

    let dalloc_bytes = fs::read(format!(
        "{}/dalloc/target/wasm32-unknown-unknown/release/dalloc.wasm",
        manifest_dir
    ))
    .map_err(|e| format!("Failed to read dalloc.wasm: {}", e))?;
    let dalloc_module = Module::new(&engine, &dalloc_bytes).map_err(|e| e.to_string())?;
    let dalloc_instance = linker
        .instantiate(&mut store, &dalloc_module)
        .map_err(|e| e.to_string())?;
    linker
        .instance(&mut store, "dalloc", dalloc_instance)
        .map_err(|e| e.to_string())?;

    let shadow_bytes = fs::read(format!(
        "{}/shadow/target/wasm32-unknown-unknown/release/shadow.wasm",
        manifest_dir
    ))
    .map_err(|e| format!("Failed to read shadow.wasm: {}", e))?;
    let shadow_module = Module::new(&engine, &shadow_bytes).map_err(|e| e.to_string())?;
    let shadow_instance = linker
        .instantiate(&mut store, &shadow_module)
        .map_err(|e| e.to_string())?;
    linker
        .instance(&mut store, "shadow", shadow_instance)
        .map_err(|e| e.to_string())?;

    let lists = dalloc_instance
        .get_memory(&mut store, "memory")
        .ok_or("Expected memory export in dalloc")?;

    let output: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let output_clone = output.clone();

    linker
        .func_wrap("env", "print", move |caller: Caller<'_, ()>, ptr: i32| {
            let data = lists.data(&caller);
            let ptr = ptr as usize;
            let length = u32::from_le_bytes(data[ptr - 4..ptr].try_into().unwrap());

            let mut string: Vec<u8> = Vec::with_capacity(length as usize);
            for i in 0..length {
                let start = ptr + (i as usize) * 8;
                string.push(data[start]);
            }

            let decoded = String::from_utf8(string).unwrap_or_else(|_| "<invalid utf8>".into());
            output_clone.lock().unwrap().push(decoded);
            Ok(())
        })
        .map_err(|e| e.to_string())?;

    let module = Module::new(&engine, &wasm_bytes).map_err(|e| e.to_string())?;
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|e| e.to_string())?;

    let main = instance
        .get_typed_func::<(i32, i64, i32), i64>(&mut store, "main")
        .map_err(|e| e.to_string())?;

    main.call(&mut store, (0, 0, 0))
        .map_err(|e| e.to_string())?;

    let result = output.lock().unwrap().clone();
    Ok(result)
}

#[derive(Debug)]
struct TestExpectation {
    output: Vec<String>,
    expect_panic: bool,
}

fn parse_test_file(content: &str) -> (String, TestExpectation) {
    let mut expected = Vec::new();
    let mut source_lines = Vec::new();
    let mut expect_panic = false;

    for line in content.lines() {
        if line.starts_with("// expect: ") {
            expected.push(line.trim_start_matches("// expect: ").to_string());
        } else if line.starts_with("// expect_panic") {
            expect_panic = true;
        } else {
            source_lines.push(line);
        }
    }

    (
        source_lines.join("\n"),
        TestExpectation {
            output: expected,
            expect_panic,
        },
    )
}

fn run_test_file(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let (source, expectation) = parse_test_file(&content);

    match run_program(&source) {
        Ok(actual) => {
            if expectation.expect_panic {
                return Err("Expected panic but program succeeded".to_string());
            }
            if actual != expectation.output {
                return Err(format!(
                    "Output mismatch:\n  Expected: {:?}\n  Actual:   {:?}",
                    expectation.output, actual
                ));
            }
            Ok(())
        }
        Err(e) => {
            if expectation.expect_panic {
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

#[test]
fn run_all_program_tests() {
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/programs");

    if !test_dir.exists() {
        println!("No tests/programs directory found, skipping");
        return;
    }

    let mut failures = Vec::new();
    let mut passed = 0;

    for entry in fs::read_dir(&test_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if path.extension().map_or(false, |e| e == "star") {
            let name = path.file_name().unwrap().to_string_lossy();
            print!("Testing {}... ", name);

            match run_test_file(&path) {
                Ok(()) => {
                    println!("OK");
                    passed += 1;
                }
                Err(e) => {
                    println!("FAILED");
                    failures.push((name.to_string(), e));
                }
            }
        }
    }

    println!("\n{} passed, {} failed", passed, failures.len());

    if !failures.is_empty() {
        println!("\nFailures:");
        for (name, err) in &failures {
            println!("\n--- {} ---\n{}", name, err);
        }
        panic!("{} test(s) failed", failures.len());
    }
}
