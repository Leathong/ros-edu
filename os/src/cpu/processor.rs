use crate::{cpu_local, task};
use alloc::sync::Arc;
use log::info;
use spin::Once;

use crate::task::{Task, context::TaskContext, context_switch};

cpu_local! {
    pub static ref PROCESSOR: Processor = Processor::new();
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

    pub fn exit_current(&self, exit_code: i32) {
        info!("exit_current: {}", exit_code);
        let current = self.current().unwrap();
        current.get_mutable_inner().exit_code = exit_code;
        current.get_mutable_inner().status = task::TaskStatus::Zombie;
    }

    pub fn switch_to_task(&mut self, next_task: Arc<Task>) {
        let mut old_ctx = match self.current.take().map(|task| task.get_inner().task_ctx) {
            Some(ctx) => ctx,
            None => self.idle_task_cx,
        };
        self.current = Some(next_task.clone());
        let inner = next_task.get_mutable_inner();
        inner.memory_set.activate();
        let ctx_ptr = &raw const (inner.task_ctx);
        
        info!("switch to task {:?}", next_task.pid);
        unsafe {
            context_switch(&mut old_ctx, ctx_ptr);
        };
    }
}

pub fn switch_to_task(task: Arc<Task>) {
    PROCESSOR.as_mut().switch_to_task(task);
}
