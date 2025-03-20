use alloc::{collections::vec_deque::VecDeque, sync::Arc};

use crate::{
    cpu::local::CpuLocalCell,
    task::{Task, schedule},
};

pub trait Lock: Sync + Send {
    fn lock(&self);
    fn unlock(&self);
}

pub struct Mutex {
    inner: CpuLocalCell<MutexInner>,
}

pub struct MutexInner {
    pub is_locked: bool,
    pub waiting_tasks: VecDeque<Arc<Task>>,
}

impl Mutex {
    pub fn new() -> Self {
        Self {
            inner: CpuLocalCell::new(MutexInner {
                is_locked: false,
                waiting_tasks: VecDeque::new(),
            }),
        }
    }
}

impl Lock for Mutex {
    fn lock(&self) {
        let inner = self.inner.get_mut();
        if inner.is_locked {
            schedule::park_current(&mut inner.waiting_tasks);
        } else {
            inner.is_locked = true;
        }
    }

    fn unlock(&self) {
        let inner = self.inner.get_mut();
        inner.is_locked = false;
        if let Some(task) = inner.waiting_tasks.pop_front() {
            schedule::add_task(task);
        }
    }
}
