pub mod context;

use core::arch::global_asm;

use context::TrapFrame;
use riscv::register::{sepc, sie, stvec};

use crate::{lang_items::print_backtrace, println, sbi::shutdown};

global_asm!(include_str!("trap.S"));

pub fn init() {
    set_trap_entry();
}

fn set_trap_entry() {
    unsafe {
        let mut value = stvec::Stvec::from_bits(trap_entry as usize);
        value.set_trap_mode(stvec::TrapMode::Direct);
        stvec::write(value);
    }
}

/// enable timer interrupt in sie CSR
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[unsafe(no_mangle)]
#[allow(unused)]
fn trap_handler(trapframe: &TrapFrame) -> ! {
    let sepc = sepc::read();
    println!("[trap] a trap occurs!");
    unsafe {
        print_backtrace(trapframe.general.s0, sepc);
    };
    shutdown(true)
}

unsafe extern "C" {
    fn trap_entry();
}
