use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());
    let mut linker = Linker::new(&engine);

    // Load and instantiate fixed-size allocator (alloc)
    let alloc_bytes = std::fs::read("alloc/target/wasm32-unknown-unknown/release/alloc.wasm")
        .expect(
            "Build alloc first: cd alloc && cargo build --target wasm32-unknown-unknown --release",
        );
    let alloc_module = Module::new(&engine, &alloc_bytes)?;
    let alloc_instance = linker.instantiate(&mut store, &alloc_module)?;
    linker.instance(&mut store, "alloc", alloc_instance)?;

    // Load and instantiate dynamic allocator (dalloc) for lists
    let dalloc_bytes = std::fs::read("dalloc/target/wasm32-unknown-unknown/release/dalloc.wasm")
        .expect("Build dalloc first: cd dalloc && cargo build --target wasm32-unknown-unknown --release");
    let dalloc_module = Module::new(&engine, &dalloc_bytes)?;
    let dalloc_instance = linker.instantiate(&mut store, &dalloc_module)?;
    linker.instance(&mut store, "dalloc", dalloc_instance)?;

    let lists = dalloc_instance
        .get_memory(&mut store, "memory")
        .expect("Expected a memory export in dalloc");

    // Host function: print
    linker.func_wrap("env", "print", move |caller: Caller<'_, ()>, ptr: i32| {
        let data = lists.data(&caller);

        let ptr = ptr as usize;
        let length = u32::from_le_bytes(data[ptr - 4..ptr].try_into().unwrap());

        let mut string: Vec<u8> = Vec::with_capacity(length as usize);

        for i in 0..length {
            let start = ptr + (i as usize) * 8;
            string.push(data[start]);
        }

        let decoded = String::from_utf8(string).unwrap();
        print!("{}\n", decoded);
        Ok(())
    })?;

    // Load Star program
    let wasm_bytes = std::fs::read("output.wasm").expect("Failed to read output.wasm");
    let module = Module::new(&engine, &wasm_bytes)?;
    let instance = linker.instantiate(&mut store, &module)?;

    // Get and call the main function
    let main = instance.get_typed_func::<(i32, i64, i32), i64>(&mut store, "main")?;
    let result = main.call(&mut store, (0, 0, 0))?;
    println!("main returned: {}", result);

    Ok(())
}
