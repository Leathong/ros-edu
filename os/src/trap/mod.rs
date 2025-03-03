pub mod context;

use crate::{println, sbi};

use riscv::register::{
    scause, sie, stval, stvec
};
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
fn trap_handler() -> ! {
    // panic!("a trap occurs!");
    let scause = scause::read();
    let stval = stval::read();
    println!("a trap occurs! scause: {}, stval: {:#x}", scause.bits(), stval);
    sbi::shutdown(true);
}

unsafe extern "C" {
    fn trap_entry();
}