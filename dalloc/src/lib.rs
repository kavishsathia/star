#![no_std]

const START: u32 = 4;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

fn memory_size() -> u32 {
    (core::arch::wasm32::memory_size(0) as u32) * 65536
}

unsafe fn read_u32(addr: u32) -> u32 {
    *(addr as *const u32)
}

unsafe fn write_u32(addr: u32, val: u32) {
    *(addr as *mut u32) = val;
}

#[no_mangle]
pub extern "C" fn dinit() {
    unsafe {
        write_u32(START, 0);
        write_u32(START + 4, 0);

        let size = memory_size();
        write_u32(START + 8, size - 20);
        write_u32(size - 4, size - 20);
    }
}

#[no_mangle]
pub extern "C" fn dalloc(ty: u32, size: u32) -> u32 {
    unsafe {
        let mut current_addr = START;
        
        while current_addr < memory_size() {
            let current_ty = read_u32(current_addr);
            let current_size = read_u32(current_addr + 8);

            if current_ty == 0 {
                if size + 16 <= current_size {
                    write_u32(current_addr, ty);
                    write_u32(current_addr + 8, size);
                    write_u32(current_addr + 12 + size, size);

                    let left = current_size - size - 16;
                    let new_start = current_addr + 16 + size;

                    write_u32(new_start, 0);
                    write_u32(new_start + 4, 0);
                    write_u32(new_start + 8, left);
                    write_u32(new_start + 12 + left, left);

                    return current_addr + 12;
                } else if size <= current_size {
                    write_u32(current_addr, ty);
                    return current_addr + 12;
                } else {
                    current_addr = current_addr + current_size + 16;
                }
            } else {
                current_addr = current_addr + current_size + 16;
            }
        }
    }

    0
}
