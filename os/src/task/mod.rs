// SPDX-License-Identifier: MPL-2.0

//! The architecture support of context switch.

mod taskid;
mod stack;
mod context;
mod utils;
pub(crate) mod processor;

use core::cell::RefCell;

use context::TaskContext;
use lazy_static::lazy_static;
use processor::PROCESSOR;
use utils::ForceSync;

core::arch::global_asm!(include_str!("switch.S"));

use crate::fs::File;
use crate::task::stack::*;
use crate::task::taskid::*;
use crate::mm::memory_set::MemorySet;
use crate::trap::context::UserContext;

use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

pub struct Task {
    pub pid: ProcessId,
    pub kstack: KernelStack,
    
    pub inner: ForceSync<RefCell<TaskInner>>,
}

impl Task {
    pub fn current_task() -> Option<Arc<Task>> {
        PROCESSOR.lock().current()
    }

    pub fn exit_current(exit_code: i32) {
        todo!()
    }

    pub fn fork(&mut self) -> Arc<Task> {
        todo!()
    }
}

pub struct TaskInner {
    pub memory_set: MemorySet,
    pub task_ctx: TaskContext,
    pub user_ctx: UserContext,

    pub parent: Option<Weak<Task>>,
    pub children: Vec<Arc<Task>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
}

impl TaskInner {
    pub fn alloc_fd(&mut self) -> usize {
        if let Some(fd) = (0..self.fd_table.len()).find(|fd| self.fd_table[*fd].is_none()) {
            fd
        } else {
            self.fd_table.push(None);
            self.fd_table.len() - 1
        }
    }
}

pub fn suspend_current_and_run_next() {
    todo!()
}

unsafe extern "C" {
    pub(crate) fn context_switch(cur: *mut TaskContext, nxt: *const TaskContext);
}
