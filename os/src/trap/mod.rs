use ros_core::{println, sbi};

use riscv::register::{
    scause, stval,
    stvec::{self, Stvec},
};
pub fn init() {
    set_trap_entry();
}

fn set_trap_entry() {
    unsafe {
        let mut value = Stvec::from_bits(trap_handler as usize);
        value.set_trap_mode(stvec::TrapMode::Direct);
        stvec::write(value);
    }
}

fn trap_handler() -> ! {
    // panic!("a trap occurs!");
    let scause = scause::read();
    let stval = stval::read();
    println!("a trap occurs! scause: {}, stval: {:#x}", scause.bits(), stval);
    sbi::shutdown();
}
