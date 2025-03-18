// SPDX-License-Identifier: MPL-2.0

//! The architecture support of context switch.

pub(crate) mod context;
pub(crate) mod schedule;
mod stack;
mod taskid;
mod utils;

use core::cell::UnsafeCell;

use crate::{
    cpu::processor::PROCESSOR,
    fs::{Stdin, Stdout},
};
use context::TaskContext;
use lazy_static::lazy_static;
use log::{debug, info, trace};
use riscv::{
    interrupt::{
        Trap,
        supervisor::{Exception, Interrupt},
    },
    register::sstatus::set_sum,
};
use schedule::add_task;
use utils::ForceSync;

core::arch::global_asm!(include_str!("switch.S"));

use crate::fs::File;
use crate::fs::OpenFlags;
use crate::mm::memory_set::KERNEL_SPACE;
use crate::mm::memory_set::MemorySet;
use crate::syscall;
use crate::task::stack::*;
use crate::task::taskid::*;
use crate::trap::context::UserContext;
use crate::{fs::open_file, timer::set_next_trigger};

use alloc::vec::Vec;
use alloc::{
    sync::{Arc, Weak},
    vec,
};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Zombie,
}

pub struct Task {
    pub pid: ProcessId,

    inner: ForceSync<UnsafeCell<TaskInner>>,
}

impl Task {
    pub fn new_with_elf(elf_data: &[u8]) -> Self {
        extern "C" fn task_kernel_entry() {
            trace!("task_kernel_entry");
            let current_task = Task::current_task().unwrap();
            let inner = current_task.get_mutable_inner();
            loop {
                trace!("run task {}", current_task.pid.value);
                inner.user_ctx.run();
                let cause = riscv::register::scause::read().cause();
                trace!(
                    "trap from task {} cause: {:?}",
                    current_task.pid.value, cause
                );
                unsafe {
                    set_sum();
                }
                match cause.try_into().unwrap() {
                    Trap::Interrupt(Interrupt::SupervisorTimer) => {
                        set_next_trigger();
                        schedule::yield_now();
                    }
                    Trap::Exception(Exception::UserEnvCall) => {
                        inner.user_ctx.sepc += 4;
                        inner.user_ctx.general.a0 = syscall::handle_syscall(
                            inner.user_ctx.get_syscall_num(),
                            inner.user_ctx.get_syscall_args(),
                        ) as usize;
                    }
                    _ => {
                        info!("Unsupported trap {:?}", cause);
                        PROCESSOR.as_mut().abort_current();
                        break;
                    }
                }

                if current_task.get_inner().status == TaskStatus::Zombie {
                    break;
                }
            }

            drop(current_task);
            // this function does not return, manually drop all the variables on the stack
            schedule::exit_current();
        }

        let kernel_space = KERNEL_SPACE.lock();
        let pid_handle = pid_alloc();
        let pt = kernel_space.get_page_table().spawn(pid_handle.value);

        let kernel_stack = KernelStack::new(pid_handle.value);
        let kernel_stack_top = kernel_stack.area.vpn_range.get_end().0 << 12;
        let mut user_ctx = UserContext::default();
        let mut task_ctx = TaskContext::default();

        let user_stack = UserStack::new();
        let user_stack_top = user_stack.area.vpn_range.get_end().0 << 12;

        let current_task = Task::current_task();
        let mut cur_pt = kernel_space.get_page_table();

        let task: Arc<Task>;
        if let Some(cur) = current_task {
            task = cur.clone();
            cur_pt = task.get_mutable_inner().memory_set.get_page_table();
        }
        // memory_set with elf
        let (mut memory_set, entry_point) = MemorySet::from_elf(elf_data, pt, cur_pt);

        memory_set.push(kernel_stack.area, None);
        memory_set.push(user_stack.area, None);

        task_ctx.set_instruction_pointer(task_kernel_entry as usize);
        task_ctx.set_stack_pointer(kernel_stack_top);

        user_ctx.set_ip(entry_point);
        user_ctx.set_sp(user_stack_top);

        debug!("create task pid {}", pid_handle.value);

        let task = Self {
            pid: pid_handle,
            inner: ForceSync::new(UnsafeCell::new(TaskInner {
                memory_set,
                task_ctx: task_ctx,
                user_ctx: user_ctx,
                parent: None,
                children: [].to_vec(),
                exit_code: 0,
                fd_table: vec![
                    // 0 -> stdin
                    Some(Arc::new(Stdin)),
                    // 1 -> stdout
                    Some(Arc::new(Stdout)),
                    // 2 -> stdout as stderr
                    Some(Arc::new(Stdout)),
                ],
                status: TaskStatus::Ready,
            })),
        };
        task
    }

    pub fn current_task() -> Option<Arc<Task>> {
        PROCESSOR.as_mut().current()
    }

    fn as_mut_ptr(&self) -> *mut TaskInner {
        self.inner.get() as *mut TaskInner
    }

    pub fn get_mutable_inner(&self) -> &mut TaskInner {
        unsafe { self.as_mut_ptr().as_mut().unwrap() }
    }

    pub fn get_inner(&self) -> &TaskInner {
        unsafe { self.as_mut_ptr().as_ref().unwrap() }
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
    pub status: TaskStatus,
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

    pub fn is_zombie(&self) -> bool {
        self.status == TaskStatus::Zombie
    }
}

unsafe extern "C" {
    pub(crate) fn context_switch(cur: *const TaskContext, nxt: *const TaskContext);
}

lazy_static! {
    ///Globle process that init user shell
    pub static ref INITPROC: Arc<Task> = Arc::new({
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        Task::new_with_elf(v.as_slice())
    });
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}
