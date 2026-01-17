#![no_std]

#[link(wasm_import_module = "alloc")]
extern "C" {
    fn read_alloc(addr: u32) -> u32;
}

#[link(wasm_import_module = "dalloc")]
extern "C" {
    fn read_dalloc(addr: u32) -> u32;
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