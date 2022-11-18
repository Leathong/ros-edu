#![no_std]
#![no_main]
#![feature(panic_info_message)]

mod lang_items;
mod sbi;
mod console;

use core::{arch::{global_asm}, slice};

global_asm!(include_str!("entry.asm"));

#[no_mangle]
pub fn rust_main() -> ! {
    let _a = 3;
    clear_bss();
    println!("hello world!");
    panic!("shutdown")
}

fn clear_bss() {
    extern "C" {
        static mut sbss:u8;
        static mut ebss:u8;
    }

    unsafe {
        let start = &mut sbss as *mut u8;
        let end = &mut ebss as *mut u8;
        let count = end as usize - start as usize;
        let slice = slice::from_raw_parts_mut(start, count);
        for i in 0..count {
            slice[i] = 0;
        }
    }
}

