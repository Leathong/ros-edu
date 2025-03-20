use crate::{sync::mutex::Mutex, task::current_process};
use alloc::sync::Arc;
pub fn sys_mutex_create() -> isize {
    let process = current_process().unwrap();
    let process_inner = process.get_mutable_inner();
    let mutex_id = process_inner.alloc_mutex();
    process_inner.mutex_list[mutex_id] = Some(Arc::new(Mutex::new()));
    mutex_id as isize
}

pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    let process = current_process().unwrap();
    let process_inner = process.get_mutable_inner();
    let mutex = process_inner.mutex_list[mutex_id].as_ref().unwrap();
    mutex.lock();
    0
}

pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    let process = current_process().unwrap();
    let process_inner = process.get_mutable_inner();
    let mutex = process_inner.mutex_list[mutex_id].as_ref().unwrap();
    mutex.unlock();
    0
}
