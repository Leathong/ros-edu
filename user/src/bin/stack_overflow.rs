#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

#[allow(unconditional_recursion)]
fn test(depth: usize) {
    if depth % 10 == 0 {
        println!("depth = {}", depth);
    }
    test(depth + 1);
}

#[no_mangle]
pub fn main() -> i32 {
    println!("It should trigger segmentation fault!");
    test(0);
    0
}
