use wasmtime::*;

fn main() -> Result<()> {
    // Read the WASM file
    let wasm_bytes = std::fs::read("output.wasm")
        .expect("Failed to read output.wasm");

    // Create the engine and store
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    // Create a linker to provide imports
    let mut linker = Linker::new(&engine);

    // Define print_i64: takes an i64, prints it
    linker.func_wrap("env", "print_i64", |value: i64| {
        println!("{}", value);
    })?;

    // Compile and instantiate the module
    let module = Module::new(&engine, &wasm_bytes)?;
    let instance = linker.instantiate(&mut store, &module)?;

    // Get and call the main function
    let main = instance.get_typed_func::<(), i64>(&mut store, "main")?;
    let result = main.call(&mut store, ())?;
    println!("main returned: {}", result);

    Ok(())
}
