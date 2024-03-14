#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use core::arch::global_asm;
use ros_core::println;

mod config;
mod mm;
mod sync;
mod lang_items;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn ros_main() -> ! {
    let _a = 3;
    mm::init();
    println!("hello world!");
    panic!("shutdown")
}

