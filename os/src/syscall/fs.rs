//! File and filesystem-related syscalls
use core::ffi::CStr;

use alloc::slice;
use alloc::vec::Vec;

use crate::fs::{OpenFlags, open_file};
use crate::mm::UserBuffer;
use crate::task::Task;
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = Task::current_task().unwrap();
    let inner = task.get_mutable_inner();

    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow

        let slice = unsafe { slice::from_raw_parts_mut::<'static, u8>(buf as *mut u8, len) };
        let mut vec = Vec::new();
        vec.push(slice);
        file.write(UserBuffer::new(vec)) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    let task = Task::current_task().unwrap();
    let inner = task.get_mutable_inner();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }

        let slice = unsafe { slice::from_raw_parts_mut::<'static, u8>(buf as *mut u8, len) };
        let mut vec = Vec::new();
        vec.push(slice);
        file.read(UserBuffer::new(vec)) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    let task = Task::current_task().unwrap();

    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(path) => path,
            Err(_) => return -1,
        }
    };
    if let Some(inode) = open_file(path, OpenFlags::from_bits(flags).unwrap()) {
        let inner = task.get_mutable_inner();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    let task = Task::current_task().unwrap();
    let inner = task.get_mutable_inner();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}
