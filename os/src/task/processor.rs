use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::task::{Task, context::TaskContext};

lazy_static! {
    pub static ref PROCESSOR: Mutex<Processor> = Mutex::new(Processor::new());
}

pub struct Processor {
    ///The task currently executing on the current processor
    current: Option<Arc<Task>>,
    ///The basic control flow of each core, helping to select and switch process
    idle_task_cx: TaskContext,
}

impl Processor {
    ///Create an empty Processor
    pub fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: TaskContext::default(),
        }
    }
    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        &mut self.idle_task_cx as *mut _
    }
    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<Task>> {
        self.current.take()
    }
    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<Task>> {
        self.current.as_ref().map(Arc::clone)
    }
}
