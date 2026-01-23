#![no_std]

#[link(wasm_import_module = "alloc")]
extern "C" {
    fn read_alloc(addr: u32) -> u32;
    fn write_alloc(addr: u32, val: u32);
    fn sweep() -> u32;
}

#[link(wasm_import_module = "dalloc")]
extern "C" {
    fn read_dalloc(addr: u32) -> u32;
    fn write_dalloc(addr: u32, val: u32);
    #[link_name = "sweep"]
    fn dsweep() -> u32;
}

const STACK_POINTER: u32 = 12;
const FRAME_POINTER: u32 = 12;
const STACK_POINTER_ADDR: u32 = 4;
const FRAME_POINTER_ADDR: u32 = 8;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

unsafe fn read_u32(addr: u32) -> u32 {
    *(addr as *const u32)
}

unsafe fn write_u32(addr: u32, val: u32) {
    *(addr as *mut u32) = val;
}

#[no_mangle]
pub extern "C" fn init() {
    unsafe {
        write_u32(STACK_POINTER_ADDR, STACK_POINTER);
        write_u32(FRAME_POINTER_ADDR, FRAME_POINTER);
    }
}

#[no_mangle]
pub extern "C" fn push(size: u32) {
    unsafe {
        let offset = size * 8 + 4;
        let sp = read_u32(STACK_POINTER_ADDR);
        let fp = read_u32(FRAME_POINTER_ADDR);

        for i in 0..size {
            write_u32(sp + (i * 8), 0);
            write_u32(sp + (i * 8) + 4, 0);
        }

        write_u32(sp + offset - 4, fp);
        write_u32(FRAME_POINTER_ADDR, sp);
        write_u32(STACK_POINTER_ADDR, sp + offset);
    }
}

#[no_mangle]
pub extern "C" fn pop() {
    unsafe {
        let sp = read_u32(STACK_POINTER_ADDR);
        let fp = read_u32(FRAME_POINTER_ADDR);

        write_u32(STACK_POINTER_ADDR, fp);
        write_u32(FRAME_POINTER_ADDR, read_u32(sp - 4));
    }
}

#[no_mangle]
pub extern "C" fn set(value: u32, index: u32, ty: u32) {
    unsafe {
        let fp = read_u32(FRAME_POINTER_ADDR);
        write_u32(fp + (index * 8), ty);
        write_u32(fp + (index * 8) + 4, value);
    }
}

#[no_mangle]
pub extern "C" fn mark() {
    unsafe {
        let sp = read_u32(STACK_POINTER_ADDR);
        let start = STACK_POINTER;
        let size = (sp - start) / 8;

        for i in 0..size {
            let ty = read_u32(start + (i * 8));
            let val = read_u32(start + (i * 8) + 4);

            if ty == 1 {
                mark_pointer(val, 1);
            } else if ty == 2 {
                mark_pointer(val, 2);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn mark_pointer(pointer: u32, memory: u32) {
    unsafe {
        if pointer == 0 {
            return;
        }
        if memory == 1 {
            if read_alloc(pointer - 4) != 1 {
                let ty = read_alloc(pointer - 8);

                write_alloc(pointer - 4, 1);

                let scount = read_u32(TYPE_TABLE_INDEX + (ty * TYPE_TABLE_RECORD_SIZE) + 8);
                for i in 0..scount {
                    let field_addr = pointer + (i * 8);
                    let field_ptr = read_alloc(field_addr);
                    mark_pointer(field_ptr, 1);
                }

                let lcount = read_u32(TYPE_TABLE_INDEX + (ty * TYPE_TABLE_RECORD_SIZE) + 12);
                for i in 0..lcount {
                    let list_addr = pointer + (scount * 8) + (i * 8);
                    let list_ptr = read_alloc(list_addr);
                    mark_pointer(list_ptr, 2);
                }
            }
        } else {
            if read_dalloc(pointer - 12) != 1 {
                let length = read_dalloc(pointer - 4);
                let ty = read_dalloc(pointer - 16);

                write_dalloc(pointer - 12, 1);

                for i in 0..length {
                    let element_addr = pointer + (i * 8);
                    let element_ptr = read_dalloc(element_addr);
                    if ty == 1 {
                        mark_pointer(element_ptr, 1);
                    } else if ty == 2 {
                        mark_pointer(element_ptr, 2);
                    }
                }
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn gc() {
    unsafe {
        mark();
        sweep();
        dsweep();
    }
}
