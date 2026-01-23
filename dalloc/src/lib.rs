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

#[no_mangle]
pub extern "C" fn read_dalloc(addr: u32) -> u32 {
    unsafe { read_u32(addr) }
}

unsafe fn write_u32(addr: u32, val: u32) {
    *(addr as *mut u32) = val;
}

#[no_mangle]
pub extern "C" fn write_dalloc(addr: u32, val: u32) {
    unsafe { write_u32(addr, val) }
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
pub extern "C" fn dfree(pointer: u32) -> u32 {
    unsafe {
        let addr = pointer - 16;
        write_u32(addr, 0);
        let mut ret = addr;

        let size = read_u32(addr + 8);
        let end = addr + 20 + size;

        if end < memory_size() {
            let next_ty = read_u32(end);
            if next_ty == 0 {
                let next_size = read_u32(end + 8);
                let combined_size = size + 20 + next_size;
                write_u32(addr + 8, combined_size);
                write_u32(addr + 12, combined_size);
                write_u32(addr + 16 + combined_size, combined_size);
            }
        }

        if addr > START {
            let prev_size = read_u32(addr - 4);
            let prev_addr = addr - 20 - prev_size;
            let prev_ty = read_u32(prev_addr);
            if prev_ty == 0 {
                let combined_size = prev_size + 20 + read_u32(addr + 8);
                write_u32(prev_addr + 8, combined_size);
                write_u32(prev_addr + 12, combined_size);
                write_u32(prev_addr + 16 + combined_size, combined_size);

                ret = prev_addr;
            }
        }
    }

    ret
}

#[no_mangle]
pub extern "C" fn sweep() -> u32 {
    unsafe {
        let mut current_addr = START;

        while current_addr < memory_size() {
            let current_ty = read_u32(current_addr);
            let current_mark = read_u32(current_addr + 4);
            let mut new_addr = current_addr;

            if current_ty != 0 && current_mark == 0 {
                new_addr = dfree(current_addr + 16);
            }

            if current_mark == 1 {
                write_u32(current_addr + 4, 0);
            }

            current_addr = new_addr + read_u32(new_addr + 8) + 20;
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
pub extern "C" fn ditoa(i: i64) -> u32 {
    unsafe {
        let mut num = if i < 0 { -i } else { i } as u64;
        let mut digits = 0;
        if i < 0 {
            digits += 1;
        }

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

        num = if i < 0 { -i } else { i } as u64;
        let offset = if i < 0 { 1 } else { 0 };
        let num_digits = digits - offset;

        if i < 0 {
            write_u64(str_addr, b'-' as u64);
        }
        for j in 0..num_digits {
            let digit = (num % 10) as u8 + b'0';
            write_u64(str_addr + ((offset + num_digits - j - 1) * 8), digit as u64);
            num /= 10;
        }

        str_addr
    }
}

#[no_mangle]
pub extern "C" fn dbtoa(i: u32) -> u32 {
    unsafe {
        if i == 0 {
            let str_addr = dalloc(2, 5);
            write_u64(str_addr, b'f' as u64);
            write_u64(str_addr + 8, b'a' as u64);
            write_u64(str_addr + 16, b'l' as u64);
            write_u64(str_addr + 24, b's' as u64);
            write_u64(str_addr + 32, b'e' as u64);
            return str_addr;
        } else {
            let str_addr = dalloc(2, 4);
            write_u64(str_addr, b't' as u64);
            write_u64(str_addr + 8, b'r' as u64);
            write_u64(str_addr + 16, b'u' as u64);
            write_u64(str_addr + 24, b'e' as u64);
            return str_addr;
        }
    }
}

#[no_mangle]
pub extern "C" fn dftoa(value: f64) -> u32 {
    unsafe {
        let int_part = value as i64;
        let frac = value - (int_part as f64);
        let frac_abs = if frac < 0.0 { -frac } else { frac };
        let frac_part = (frac_abs * 1000000.0 + 0.5) as u64;

        let int_str = ditoa(int_part);
        let dot_str = dalloc(2, 1);
        write_u64(dot_str, b'.' as u64);

        let frac_str = ditoa(frac_part as i64);
        let frac_len = read_u32(frac_str - 4);

        let zeros_needed = 6 - frac_len;
        let padded_frac = if zeros_needed > 0 {
            let zeros = dalloc(2, zeros_needed);
            for i in 0..zeros_needed {
                write_u64(zeros + i * 8, b'0' as u64);
            }
            dconcat(zeros, frac_str)
        } else {
            frac_str
        };

        let with_dot = dconcat(int_str, dot_str);
        dconcat(with_dot, padded_frac)
    }
}
