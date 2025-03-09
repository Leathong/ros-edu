use core::{arch::asm, panic::PanicInfo};
use crate::{println, sbi::shutdown};

#[allow(unused)]
pub unsafe fn print_backtrace(fp: usize, pc: usize) {
    println!("\nbacktrace:");
    unsafe {
        let mut fp = fp;
        let mut idx = 1;
        println!("\t0:\t{:#x}", pc);
        while fp != 0 {
            let ra = *(fp as *const usize).offset(-1) - 4;
            let lfp = *(fp as *const usize).offset(-2);

            println!("\t{}:\t{:#x}", idx, ra);
            fp = lfp;
            idx += 1;
        }
    }
    println!("backtrace end\n");
}

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{}\n{}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message())
    }
    unsafe {
        let mut fp: usize;
        let mut pc: usize;
        asm!("mv {0}, fp", "auipc {1}, 0", out(reg) fp, out(reg) pc);
        print_backtrace(fp, pc);
    }
    shutdown(true)
}
