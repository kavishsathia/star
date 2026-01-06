use wasmtime::*;

fn main() -> Result<()> {
    let engine = Engine::default();
    let mut store = Store::new(&engine, ());

    let wasm_bytes = std::fs::read("target/wasm32-unknown-unknown/release/alloc.wasm")
        .expect("Build first: cargo build --target wasm32-unknown-unknown --release");

    let module = Module::new(&engine, &wasm_bytes)?;
    let instance = Instance::new(&mut store, &module, &[])?;

    // Get exported functions
    let init = instance.get_typed_func::<(), ()>(&mut store, "init")?;
    let register = instance.get_typed_func::<u32, ()>(&mut store, "register")?;
    let falloc = instance.get_typed_func::<u32, u32>(&mut store, "falloc")?;

    // Initialize allocator
    init.call(&mut store, ())?;
    println!("init() done");

    // Register type 0 with size 16 (e.g., a struct with 4 i32 fields)
    register.call(&mut store, 16)?;
    println!("register(16) done - type 0");

    // Register type 1 with size 8 (e.g., a struct with 2 i32 fields)
    register.call(&mut store, 8)?;
    println!("register(8) done - type 1");

    // Allocate some objects
    let ptr1 = falloc.call(&mut store, 0)?;
    println!("falloc(0) = {} (type 0, first alloc)", ptr1);

    let ptr2 = falloc.call(&mut store, 0)?;
    println!("falloc(0) = {} (type 0, second alloc)", ptr2);

    let ptr3 = falloc.call(&mut store, 1)?;
    println!("falloc(1) = {} (type 1, first alloc)", ptr3);

    // Verify allocations are sequential within slab
    // Type 0: block_size = 8 (header) + 16 (data) = 24
    // Type 1: block_size = 8 (header) + 8 (data) = 16
    println!("\nExpected:");
    println!("  ptr2 - ptr1 = 24 (block size for type 0)");
    println!("Actual:");
    println!("  ptr2 - ptr1 = {}", ptr2 - ptr1);

    // Allocate 30 more type 0 objects to exhaust first slab
    println!("\nAllocating 30 more type 0 objects...");
    for i in 0..30 {
        let ptr = falloc.call(&mut store, 0)?;
        if i == 29 {
            println!("falloc(0) = {} (32nd allocation, still in first slab)", ptr);
        }
    }

    // This should trigger a new slab
    let ptr_new_slab = falloc.call(&mut store, 0)?;
    println!("falloc(0) = {} (33rd allocation, new slab!)", ptr_new_slab);

    Ok(())
}
