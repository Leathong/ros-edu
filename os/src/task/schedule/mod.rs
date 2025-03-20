use core::arch::asm;

use alloc::{collections::vec_deque::VecDeque, sync::Arc};
use lazy_static::lazy_static;
use log::debug;
use spin::Mutex;

use crate::{
    cpu::processor,
    task::{Task, TaskStatus, current_task},
};

enum ReschedAction {
    /// Keep running current task and do nothing.
    DoNothing,
    /// Loop until finding a task to swap out the current.
    Retry,
    /// Switch to target task.
    SwitchTo(Arc<Task>),
}

pub struct TaskManager {
    ready_queue: VecDeque<Arc<Task>>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
        }
    }
    pub fn add_task(&mut self, task: Arc<Task>) {
        self.ready_queue.push_back(task);
    }
}

lazy_static! {
    pub static ref TASK_MANAGER: Mutex<TaskManager> = Mutex::new(TaskManager::new());
}

pub fn add_task(task: Arc<Task>) {
    TASK_MANAGER.lock().add_task(task);
}

pub fn park_current(waiting_queue: &mut VecDeque<Arc<Task>>) {
    reschedule(|rq| {
        let current = rq.pop_front().unwrap();
        current.get_mutable_inner().user_ctx.sepc -= 4;
        current.get_mutable_inner().status = TaskStatus::Waiting;
        waiting_queue.push_back(current);
        ReschedAction::DoNothing
    });
}

pub fn exit_current() {
    reschedule(|rq| {
        rq.pop_front();
        if let Some(next) = rq.front() {
            ReschedAction::SwitchTo(next.clone())
        } else {
            ReschedAction::Retry
        }
    });
}

pub fn yield_now() {
    debug!("yield");
    reschedule(|rq| {
        if let Some(current_task) = current_task() {
            if current_task.get_inner().status != TaskStatus::Waiting {
                rq.pop_front();
                rq.push_back(current_task);
            }
        }
        if let Some(next_task) = rq.front() {
            ReschedAction::SwitchTo(next_task.clone())
        } else {
            ReschedAction::Retry
        }
    })
}

fn reschedule<F>(mut get_action: F)
where
    F: FnMut(&mut VecDeque<Arc<Task>>) -> ReschedAction,
{
    let next_task = loop {
        let action = get_action(&mut TASK_MANAGER.lock().ready_queue);
        match action {
            ReschedAction::DoNothing => {
                debug!("do nothing");
                return;
            }
            ReschedAction::Retry => {
                debug!("retry");
                unsafe {
                    asm!("wfi");
                }
                continue;
            }
            ReschedAction::SwitchTo(next_task) => {
                break next_task;
            }
        };
    };

    processor::switch_to_task(next_task);
}
