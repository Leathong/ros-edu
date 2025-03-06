use core::{arch::asm, panic::PanicInfo};
use crate::{println, sbi::shutdown};

#[allow(unused)]
pub unsafe fn print_backtrace(fp: usize) {
    println!("\nbacktrace:");
    unsafe {
        let mut fp = fp;
        let mut idx = 0;
        while fp != 0 {
            let ra = *(fp as *const usize).offset(-1) - 4;
            let lfp = *(fp as *const usize).offset(-2);
            idx += 1;

            println!("\t{}:\tfp: {:#x} lfp: {:#x} ra: {:#x}", idx, fp, lfp, ra);
            fp = lfp;
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
        asm!("mv {0}, fp", out(reg) fp);
        print_backtrace(fp);
    }
    shutdown(true)
}
