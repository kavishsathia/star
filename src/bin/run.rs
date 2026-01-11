use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // Host function: print_i64
    linker.func_wrap("env", "print_i64", |value: i64| {
        println!("{}", value);
    })?;

    // Load and instantiate fixed-size allocator (alloc)
    let alloc_bytes = std::fs::read("alloc/target/wasm32-unknown-unknown/release/alloc.wasm")
        .expect("Build alloc first: cd alloc && cargo build --target wasm32-unknown-unknown --release");
    let alloc_module = Module::new(&engine, &alloc_bytes)?;
    let alloc_instance = linker.instantiate(&mut store, &alloc_module)?;
    linker.instance(&mut store, "alloc", alloc_instance)?;

    // Load and instantiate dynamic allocator (dalloc) for lists
    let dalloc_bytes = std::fs::read("dalloc/target/wasm32-unknown-unknown/release/dalloc.wasm")
        .expect("Build dalloc first: cd dalloc && cargo build --target wasm32-unknown-unknown --release");
    let dalloc_module = Module::new(&engine, &dalloc_bytes)?;
    let dalloc_instance = linker.instantiate(&mut store, &dalloc_module)?;
    linker.instance(&mut store, "dalloc", dalloc_instance)?;

    // Load Star program
    let wasm_bytes = std::fs::read("output.wasm")
        .expect("Failed to read output.wasm");
    let module = Module::new(&engine, &wasm_bytes)?;
    let instance = linker.instantiate(&mut store, &module)?;

    // Get and call the main function
    let main = instance.get_typed_func::<(i32,i64,i32), i64>(&mut store, "main")?;
    let result = main.call(&mut store, (0, 0, 0))?;
    println!("main returned: {}", result);

    Ok(())
}
