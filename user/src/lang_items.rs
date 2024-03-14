use core::panic::PanicInfo;
use ros_core::{println, sbi::shutdown};

#[panic_handler]
pub fn panic_handler(info: &PanicInfo) -> ! {
    if let Some(location) = info.location() {
        println!(
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println!("Panicked: {}", info.message())
    }
    shutdown()
}
