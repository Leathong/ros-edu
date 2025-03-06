pub mod context;

use core::arch::{asm, global_asm, naked_asm};

use context::TrapFrame;
use riscv::register::{scause, sepc, sie, stval, stvec};

use crate::{lang_items::print_backtrace, println, sbi::shutdown};

const XLENB: usize = 8;

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
    let scause = scause::read();
    let stval = stval::read();

    println!(
        "[trap] a trap occurs! scause: {}, stval: {:#x} sepc: {:#x}",
        scause.bits(),
        stval,
        sepc::read()
    );
    unsafe {
        print_backtrace(trapframe.general.s0);
    };
    shutdown(true)
}

unsafe extern "C" {
    fn trap_entry();
}
