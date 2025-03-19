#![no_std]
#![no_main]

use user_lib::shutdown;
extern crate user_lib;

#[no_mangle]
pub fn main() -> i32 {
    shutdown();
    0
}
