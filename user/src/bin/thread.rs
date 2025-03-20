#![no_std]
#![no_main]

use user_lib::{exit, mutex_create, mutex_lock, mutex_unlock, sleep, thread_create, waittid};

#[macro_use]
extern crate user_lib;

static mut COUNTER: usize = 0;

fn func(tid: usize) {
    unsafe {
        while COUNTER < 1000 {
            mutex_lock(0);
            COUNTER += 1;
            println!("thread {} counter {}", tid, COUNTER);
            mutex_unlock(0);
            sleep(10);
        }
    };

    exit(0);
}

#[no_mangle]
pub fn main() -> i32 {
    let tid = thread_create(func as usize, 1);
    let tid1 = thread_create(func as usize, 2);

    println!("thread {} {} created", tid, tid1);

    assert!(mutex_create() == 0);
    assert!(tid > 0 && tid1 > 0);
    println!("wait tid {}", tid);
    waittid(tid as usize);
    println!("wait tid {}", tid1);
    waittid(tid1 as usize);
    0
}
