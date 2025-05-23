use core::ffi::CStr;

use crate::cpu::processor::PROCESSOR;
use crate::fs::{OpenFlags, open_file};
use crate::task::schedule::{self, add_task};
use crate::task::{Task, TaskStatus, current_task};
use crate::timer::get_time_ms;
use alloc::sync::Arc;

pub fn sys_exit(exit_code: i32) -> isize {
    PROCESSOR.as_mut().exit_current(exit_code);
    0
}

pub fn sys_abort() -> isize {
    PROCESSOR.as_mut().abort_current();
    0
}

pub fn sys_yield() -> isize {
    PROCESSOR.as_mut().yield_current();
    0
}

pub fn sys_get_time() -> isize {
    get_time_ms() as isize
}

pub fn sys_getpid() -> isize {
    current_task().unwrap().taskid.value as isize
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
        let current = current_task().unwrap();
        let new_task = Task::new_with_elf(all_data.as_slice());
        let pid = new_task.taskid.value;
        new_task.get_mutable_inner().parent = Some(Arc::downgrade(&current));
        current.get_mutable_inner().children.push(new_task.clone());
        add_task(new_task);
        pid as isize
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    let current = current_task().unwrap();
    // find a child process

    let inner = current.get_mutable_inner();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.taskid.value)
    {
        return -1;
    }
    let pair = inner
        .children
        .iter()
        .enumerate()
        .find(|(_, p)| pid == -1 || pid as usize == p.taskid.value);
    if let Some((idx, child)) = pair {
        if child.get_inner().status != TaskStatus::Zombie {
            current.get_mutable_inner().status = TaskStatus::Waiting;
            schedule::park_current(&mut child.get_mutable_inner().waiting_tasks);
            return -1;
        }

        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.taskid.value;
        let exit_code = child.get_inner().exit_code;
        unsafe {
            *exit_code_ptr = exit_code;
        }
        found_pid as isize
    } else {
        -2
    }
}
