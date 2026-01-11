use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let wasm_bytes = std::fs::read("target/wasm32-unknown-unknown/release/dalloc.wasm")
        .expect("Build first: cargo build --target wasm32-unknown-unknown --release");

    let module = Module::new(&engine, &wasm_bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // Get exported functions
    let dinit = instance.get_typed_func::<(), ()>(&mut store, "dinit")?;
    let dalloc = instance.get_typed_func::<(u32, u32), u32>(&mut store, "dalloc")?;

    // Get memory for inspection
    let memory = instance.get_memory(&mut store, "memory").expect("memory export");

    // Initialize allocator
    dinit.call(&mut store, ())?;
    println!("dinit() done");
    println!("Memory size: {} bytes ({} pages)", memory.data_size(&store), memory.size(&store));

    // Allocate a 32-byte block with type 3 (int)
    let ptr1 = dalloc.call(&mut store, (3, 32))?;
    println!("\ndalloc(ty=3, size=32) = {}", ptr1);
    println!("  Expected: 16 (START=4 + header=12)");

    // Allocate another 32-byte block
    let ptr2 = dalloc.call(&mut store, (3, 32))?;
    println!("\ndalloc(ty=3, size=32) = {}", ptr2);
    println!("  Expected: {} (ptr1 + size=32 + overhead=16)", ptr1 + 32 + 16);
    println!("  Actual gap: {}", ptr2 - ptr1);

    // Allocate a smaller block
    let ptr3 = dalloc.call(&mut store, (2, 8))?;
    println!("\ndalloc(ty=2, size=8) = {}", ptr3);
    println!("  Expected: {} (ptr2 + 32 + 16)", ptr2 + 32 + 16);

    // Try to allocate a huge block (should fail, return 0)
    let ptr_fail = dalloc.call(&mut store, (3, 100000))?;
    println!("\ndalloc(ty=3, size=100000) = {}", ptr_fail);
    println!("  Expected: 0 (allocation failure)");

    // Inspect memory layout
    println!("\n--- Memory inspection ---");
    let data = memory.data(&store);

    // First block header (at START=4)
    println!("Block 1 header at 4:");
    println!("  type: {}", u32::from_le_bytes(data[4..8].try_into().unwrap()));
    println!("  gc:   {}", u32::from_le_bytes(data[8..12].try_into().unwrap()));
    println!("  size: {}", u32::from_le_bytes(data[12..16].try_into().unwrap()));

    Ok(())
}
