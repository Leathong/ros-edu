use core::{arch::asm, panic::{self, PanicInfo}};
// use crate::{println, sbi::shutdown};

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    // if let Some(location) = info.location() {
    //     println!(
    //         "Panicked at {}:{} {}",
    //         location.file(),
    //         location.line(),
    //         info.message()
    //     );
    // } else {
    //     println!("Panicked: {}", info.message())
    // }
    // shutdown()
    panic!("{}", info.message());
}
