use alloc::sync::Arc;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::task::{Task, context::TaskContext};

use super::context_switch;

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

    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<Task>> {
        self.current.as_ref().map(Arc::clone)
    }

    pub fn switch_to_task(&mut self, next_task: Arc<Task>) {
        let mut old_ctx = match self.current.take().map(|task| {
            task.get_inner().task_ctx
        }) {
            Some(ctx) => ctx,
            None => self.idle_task_cx,
        };
        self.current = Some(next_task.clone());
        unsafe { context_switch(&mut old_ctx, &next_task.get_inner().task_ctx) };
    }
}

pub fn switch_to_task(task: Arc<Task>) {
    PROCESSOR.lock().switch_to_task(task);
}
