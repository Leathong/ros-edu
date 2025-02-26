#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

use core::arch::global_asm;
use fdt::Fdt;
use ros_core::{println, sbi::shutdown};

mod config;
mod mm;
mod lang_items;
mod trap;

global_asm!(include_str!("entry.asm"));

#[unsafe(no_mangle)]
pub fn ros_main(_hartid: usize, dtb_addr: usize) -> ! {
    trap::init();

    let fdt = parse_dtb(dtb_addr);
    mm::init(&fdt);
    println!("hello world!");
    shutdown();
}

fn parse_dtb(dtb_addr: usize) -> Fdt<'static> {
    unsafe {
        let fdt = match fdt::Fdt::from_ptr(dtb_addr as *mut u8) {
            Ok(fdt) => fdt,
            Err(_) => panic!("invalid device tree"),
        };
        println!("This is a devicetree representation of a {}", fdt.root().model());
        for cap in fdt.root().compatible().all() {
            println!("...and is compatible with: {}", cap);
        }
        println!("...and has {} CPU(s)", fdt.cpus().count());

        let chosen = fdt.chosen();
        if let Some(bootargs) = chosen.bootargs() {
            println!("The bootargs are: {:?}", bootargs);
        }

        if let Some(stdout) = chosen.stdout() {
            println!("It would write stdout to: {}", stdout.name);
        }

        let soc = fdt.find_node("/soc");
        println!("Does it have a `/soc` node? {}", if soc.is_some() { "yes" } else { "no" });
        if let Some(soc) = soc {
            println!("...and it has the following children:");
            for child in soc.children() {
                println!("    {}", child.name);
            }
        }

        fdt
    }
}

