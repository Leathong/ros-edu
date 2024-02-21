#![no_std]
#![no_main]
#![feature(panic_info_message)]

use kernel::println;

use core::arch::global_asm;

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
        (sbss as usize .. ebss as usize).for_each(|a| {
            (a as *mut u8).write_volatile(0);
        });
    }
}

