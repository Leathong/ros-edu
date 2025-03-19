#![no_std]
#![no_main]
#![feature(alloc_error_handler)]
#![feature(naked_functions)]

extern crate alloc;

mod config;
mod console;
mod cpu;
mod drivers;
mod fs;
mod lang_items;
mod logger;
mod mm;
mod sbi;
mod syscall;
mod task;
mod timer;
mod trap;

use crate::sbi::shutdown;
use core::arch::global_asm;
use fdt::Fdt;
use log::info;
use task::schedule::yield_now;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn ros_main(_hartid: usize, dtb_addr: usize) -> ! {
    trap::init();
    let _ = logger::init();
    let fdt = mm::init(dtb_addr);

    //FIXME: if not init again, the subsequent log will be lost, I don't know why yet.
    let _ = logger::init();
    walk_dt(fdt);

    trap::init();
    // trap::enable_timer_interrupt();
    // timer::set_next_trigger();

    fs::list_apps();
    task::add_initproc();

    yield_now();
    println!("hello world!");
    shutdown(false);
}

fn walk_dt(fdt: Fdt) {
    for node in fdt.all_nodes() {
        if let Some(compatible) = node.compatible() {
            info!("\t{}", node.name);
            if compatible.all().any(|s| s == "virtio,mmio") {
                drivers::block::virtio_probe(node);
            }
        }
    }
}
