use crate::task::{
    Task, TaskStatus, current_process, current_task,
    schedule::{self, add_task},
};

pub fn sys_thread_create(entry: usize, arg: usize) -> isize {
    let process = current_process().unwrap();
    // create a new thread
    let thread = Task::new_thread(&process, entry, arg);
    let tid = thread.tid;
    add_task(thread);
    tid as isize
}

pub fn sys_gettid() -> isize {
    current_task().unwrap().tid as isize
}

/// thread does not exist, return -1
/// thread has not exited yet, return -2
/// otherwise, return thread's exit code
pub fn sys_waittid(tid: usize) -> i32 {
    let current = current_task().unwrap();
    // a thread cannot wait for itself
    if current.tid == tid {
        return -1;
    }
    let process = current_process().unwrap();
    let process_inner = process.get_mutable_inner();
    let waited_task = process_inner.threads[tid].as_ref();
    if let Some(waited_task) = waited_task {
        if waited_task.get_inner().status != TaskStatus::Zombie {
            schedule::park_current(&mut (waited_task.get_mutable_inner().waiting_tasks));
            return -1;
        }
        return waited_task.get_inner().exit_code;
    } else {
        // waited thread does not exist
        return -1;
    }
}
