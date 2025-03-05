use core::ffi::CStr;

use crate::fs::{open_file, OpenFlags};
use crate::task::schedule::add_task;
use crate::task::{
    self, schedule, Task
};
use crate::timer::get_time_ms;
use alloc::sync::Arc;

pub fn sys_exit(exit_code: i32) -> ! {
    let current = Task::current_task().unwrap();
    current.get_mutable_inner().exit_code = exit_code;
    current.get_mutable_inner().status = task::TaskStatus::Zombie;
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    schedule::yield_now();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    Task::current_task().unwrap().pid.value as isize
}

pub fn sys_spawn(path: *const u8) -> isize {
    let path = unsafe {
        match CStr::from_ptr(path).to_str() {
            Ok(path) => path,
            Err(_) => return -1,
        }
    };
    if let Some(app_inode) = open_file(path, OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let current = Task::current_task().unwrap();
        let new_task = Arc::new(Task::new(all_data.as_slice()));
        new_task.get_mutable_inner().parent = Some(Arc::downgrade(&current));
        current.get_mutable_inner().children.push(new_task.clone());
        add_task(new_task.clone());
        new_task.pid.value as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let task = Task::current_task().unwrap();
    // find a child process

    let mut inner = task.get_mutable_inner();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.pid.value)
    {
        return -1;
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        p.get_inner().is_zombie() && (pid == -1 || pid as usize == p.pid.value)
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.pid.value;
        let exit_code = child.get_inner().exit_code;
        unsafe {
            *exit_code_ptr = exit_code;
        }
        found_pid as isize
    } else {
        -2
    }
}
