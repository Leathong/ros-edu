pub mod context;

use core::arch::{asm, global_asm, naked_asm};

use context::TrapFrame;
use riscv::register::{scause, sepc, sie, stval, stvec};

use crate::{println, sbi::shutdown};

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

#[allow(unused)]
unsafe fn print_backtrace(fp: usize) {
    println!("backtrace:");
    unsafe {
        let mut fp = fp;
        let mut idx = 0;
        while fp != 0 {
            let ra = *(fp as *const usize).offset(-1);
            let lfp = *(fp as *const usize).offset(-2);
            idx += 1;

            println!("\t{}:\tfp: {:#x} lfp: {:#x} ra: {:#x}", idx, fp, lfp, ra);
            fp = lfp;
        }
    }
}

#[unsafe(no_mangle)]
#[allow(unused)]
fn trap_handler(trapframe: &TrapFrame) -> ! {
    let scause = scause::read();
    let stval = stval::read();
    // panic!(
    //     "a trap occurs! scause: {}, stval: {:#x} sepc: {:#x}",
    //     scause.bits(),
    //     stval,
    //     sepc::read() - 4
    // );

    println!(
        "a trap occurs! scause: {}, stval: {:#x} sepc: {:#x}",
        scause.bits(),
        stval,
        sepc::read() - 4
    );
    unsafe {
        print_backtrace(trapframe.general.s0);
    };
    shutdown(true)
}

unsafe extern "C" {
    fn trap_entry();
}
