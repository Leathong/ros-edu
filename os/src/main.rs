#![no_std]
#![no_main]

use core::arch::global_asm;

#[macro_use]
extern crate bitflags;

mod config;
mod mm;
mod sync;
mod console;
mod lang_items;
mod sbi;

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
        static mut sbss:usize;
        static mut ebss:usize;
    }

    unsafe {
        (sbss .. ebss).for_each(|a| {
            (a as *mut u8).write_volatile(0);
        });
    }
}

