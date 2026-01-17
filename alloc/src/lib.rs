#![no_std]

const TYPE_TABLE_INDEX: u32 = 8;
const TYPE_TABLE_RECORD_SIZE: u32 = 16;
const HEADER_SIZE: u32 = 8;
const BUMP_PTR_ADDR: u32 = 4;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
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
