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

unsafe fn read_u64(addr: u32) -> u64 {
    *(addr as *const u64)
}

unsafe fn write_u64(addr: u32, val: u64) {
    *(addr as *mut u64) = val;
}

#[no_mangle]
pub extern "C" fn dinit() {
    unsafe {
        write_u32(START, 0);
        write_u32(START + 4, 0);

        let size = memory_size();
        write_u32(START + 8, size - 24);
        write_u32(START + 12, size - 24);
        write_u32(size - 4, size - 24);
    }
}

#[no_mangle]
pub extern "C" fn dalloc(ty: u32, length: u32) -> u32 {
    unsafe {
        let size = length * 8;

        let mut current_addr = START;

        while current_addr < memory_size() {
            let current_ty = read_u32(current_addr);
            let current_size = read_u32(current_addr + 8);

            if current_ty == 0 {
                if size + 20 <= current_size {
                    write_u32(current_addr, ty);
                    write_u32(current_addr + 8, size);
                    write_u32(current_addr + 12, length);
                    write_u32(current_addr + 16 + size, size);

                    let left = current_size - size - 20;
                    let new_start = current_addr + 20 + size;

                    write_u32(new_start, 0);
                    write_u32(new_start + 4, 0);
                    write_u32(new_start + 8, left);
                    write_u32(new_start + 12, left);
                    write_u32(new_start + 16 + left, left);

                    return current_addr + 16;
                } else if size <= current_size {
                    write_u32(current_addr, ty);
                    return current_addr + 16;
                } else {
                    current_addr = current_addr + current_size + 20;
                }
            } else {
                current_addr = current_addr + current_size + 20;
            }
        }
    }

    0
}

#[no_mangle]
pub extern "C" fn dconcat(first: u32, second: u32) -> u32 {
    unsafe {
        let ty = read_u32(first - 16);
        let first_len = read_u32(first - 4);
        let second_len = read_u32(second - 4);

        let new_len = first_len + second_len;

        let new_addr = dalloc(ty, new_len);
        if new_addr == 0 {
            return 0;
        }

        for i in 0..first_len {
            let val = read_u64(first + (i * 8));
            write_u64(new_addr + (i * 8), val);
        }

        for i in 0..second_len {
            let val = read_u64(second + (i * 8));
            write_u64(new_addr + ((first_len + i) * 8), val);
        }

        new_addr
    }
}

#[no_mangle]
pub extern "C" fn dslice(ptr: u32, start: u32, end: u32) -> u32 {
    unsafe {
        let ty = read_u32(ptr - 16);
        let new_len = end - start;

        let new_addr = dalloc(ty, new_len);
        if new_addr == 0 {
            return 0;
        }

        for i in 0..new_len {
            let val = read_u64(ptr + ((start + i) * 8));
            write_u64(new_addr + (i * 8), val);
        }

        new_addr
    }
}

#[no_mangle]
pub extern "C" fn din_u64(elem: u64, list: u32) -> u32 {
    unsafe {
        let length = read_u32(list - 4);

        for i in 0..length {
            let val = read_u64(list + (i * 8));
            if val == elem {
                return 1;
            }
        }

        return 0;
    }
}

#[no_mangle]
pub extern "C" fn deq(first: u32, second: u32) -> u32 {
    unsafe {
        let firstl = read_u32(first - 4);
        let secondl = read_u32(second - 4);

        if firstl != secondl {
            return 0;
        }

        for i in 0..firstl {
            let vala = read_u64(first + (i * 8));
            let valb = read_u64(second + (i * 8));
            if vala != valb {
                return 0;
            }
        }

        return 1;
    }
}

#[no_mangle]
pub extern "C" fn ditoa(i: u64) -> u32 {
    unsafe {
        let mut num = i;
        let mut digits = 0;

        if num == 0 {
            digits = 1;
        } else {
            while num > 0 {
                digits += 1;
                num /= 10;
            }
        }

        let str_addr = dalloc(2, digits);
        if str_addr == 0 {
            return 0;
        }

        num = i;
        for j in 0..digits {
            let digit = (num % 10) as u8 + b'0';
            write_u64(str_addr + ((digits - j - 1) * 8), digit as u64);
            num /= 10;
        }

        str_addr
    }
}
