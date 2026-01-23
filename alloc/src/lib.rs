#![no_std]

const TYPE_TABLE_INDEX: u32 = 12;
const TYPE_TABLE_RECORD_SIZE: u32 = 16;
const HEADER_SIZE: u32 = 8;
const BUMP_PTR_ADDR: u32 = 8;
const DATA_START_ADDR: u32 = 4;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn alloc_memory_size() -> u32 {
    (core::arch::wasm32::memory_size(0) as u32) * 65536
}

unsafe fn read_u32(addr: u32) -> u32 {
    *(addr as *const u32)
}

#[no_mangle]
pub extern "C" fn read_alloc(addr: u32) -> u32 {
    unsafe { read_u32(addr) }
}

unsafe fn write_u32(addr: u32, val: u32) {
    *(addr as *mut u32) = val;
}

#[no_mangle]
pub extern "C" fn write_alloc(addr: u32, val: u32) {
    unsafe { write_u32(addr, val) }
}

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        write_u32(BUMP_PTR_ADDR, TYPE_TABLE_INDEX);
    }
}

#[no_mangle]
pub extern "C" fn register(size: u32, struct_count: u32, list_count: u32) {
    unsafe {
        let bump = read_u32(BUMP_PTR_ADDR);
        write_u32(BUMP_PTR_ADDR, bump + TYPE_TABLE_RECORD_SIZE);
        write_u32(DATA_START_ADDR, bump + TYPE_TABLE_RECORD_SIZE);

        write_u32(bump, size);
        write_u32(bump + 4, 0);
        write_u32(bump + 8, struct_count);
        write_u32(bump + 12, list_count);
    }
}

#[no_mangle]
pub extern "C" fn falloc(id: u32) -> u32 {
    unsafe {
        let start: u32 = TYPE_TABLE_INDEX + (id * TYPE_TABLE_RECORD_SIZE);
        let size: u32 = read_u32(start);
        let mut free: u32 = read_u32(start + 4);

        if free == 0 {
            let bump = read_u32(BUMP_PTR_ADDR);

            let block_size = HEADER_SIZE + size;
            let slab_size = 32 * block_size;

            if bump + slab_size > alloc_memory_size() {
                return 0;
            }

            write_u32(BUMP_PTR_ADDR, bump + slab_size);

            for i in 0..31 {
                let addr = bump + (i * block_size);
                write_u32(addr, id);
                write_u32(addr + HEADER_SIZE, addr + block_size);
            }

            let addr = bump + (31 * block_size);
            write_u32(addr, id);
            write_u32(addr + HEADER_SIZE, 0);

            free = bump;
        }

        let next: u32 = read_u32(free + HEADER_SIZE);
        write_u32(start + 4, next);
        free + HEADER_SIZE
    }
}

#[no_mangle]
pub extern "C" fn ffree(pointer: u32) -> u32 {
    unsafe {
        let addr = pointer - HEADER_SIZE;
        let id = read_u32(addr);

        let start: u32 = TYPE_TABLE_INDEX + (id * TYPE_TABLE_RECORD_SIZE);
        let free: u32 = read_u32(start + 4);

        write_u32(addr + HEADER_SIZE, free);
        write_u32(start + 4, addr);

        0
    }
}

#[no_mangle]
pub extern "C" fn sweep() -> u32 {
    unsafe {
        let data_start = read_u32(DATA_START_ADDR);
        let num_types = (data_start - TYPE_TABLE_INDEX) / TYPE_TABLE_RECORD_SIZE;

        for t in 0..num_types {
            write_u32(TYPE_TABLE_INDEX + (t * TYPE_TABLE_RECORD_SIZE) + 4, 0);
        }

        let mut current_addr = data_start;
        let bump_ptr = read_u32(BUMP_PTR_ADDR);

        while current_addr < bump_ptr {
            let ty = read_u32(current_addr);
            let current_size = read_u32(TYPE_TABLE_INDEX + (ty * TYPE_TABLE_RECORD_SIZE));

            for i in 0..32 {
                let block_addr = current_addr + (i * (HEADER_SIZE + current_size));
                let is_marked = read_u32(block_addr + 4);

                if is_marked == 1 {
                    write_u32(block_addr + 4, 0);
                } else {
                    ffree(block_addr + HEADER_SIZE);
                }
            }

            current_addr += 32 * (HEADER_SIZE + current_size);
        }

        0
    }
}
