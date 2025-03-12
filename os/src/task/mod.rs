// SPDX-License-Identifier: MPL-2.0

//! The architecture support of context switch.

pub(crate) mod context;
pub(crate) mod schedule;
mod stack;
mod taskid;
mod utils;

use core::cell::{Ref, RefCell, RefMut, UnsafeCell};

use context::TaskContext;
use lazy_static::lazy_static;
use log::info;
use crate::cpu::processor::PROCESSOR;
use riscv::interrupt::{Trap, supervisor::{Exception, Interrupt}};
use schedule::add_task;
use utils::ForceSync;

core::arch::global_asm!(include_str!("switch.S"));

use crate::{fs::open_file, timer::set_next_trigger};
use crate::fs::File;
use crate::fs::OpenFlags;
use crate::mm::memory_set::MemorySet;
use crate::mm::memory_set::KERNEL_SPACE;
use crate::println;
use crate::syscall;
use crate::task::stack::*;
use crate::task::taskid::*;
use crate::trap::context::UserContext;

use alloc::sync::{Arc, Weak};
use alloc::vec::Vec;

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Zombie,
}

pub struct Task {
    pub pid: ProcessId,
    pub kstack: KernelStack,

    inner: ForceSync<UnsafeCell<TaskInner>>,
}

impl Task {

    pub fn new(elf_data: &[u8]) -> Self {
        extern "C" fn task_kernel_entry() {
            info!("task_kernel_entry");
            let current_task = Task::current_task().unwrap();
            info!("");
            let inner = current_task.get_mutable_inner();
            info!("");
            loop {
                info!("run task {}", current_task.pid.value);
                inner.user_ctx.run();
                let cause = riscv::register::scause::read().cause();
                info!("trap from task {} cause: {:?}", current_task.pid.value, cause);
                match cause.try_into().unwrap() {
                    Trap::Interrupt(Interrupt::SupervisorTimer) => {
                        set_next_trigger();
                        schedule::yield_now();
                    },
                    Trap::Exception(Exception::UserEnvCall) => {
                        inner.user_ctx.sepc += 4;
                        syscall::handle_syscall(inner.user_ctx.get_syscall_num(), inner.user_ctx.get_syscall_args());
                    }
                    _ => {
                        PROCESSOR.as_mut().exit_current(i32::MIN);
                        println!("Unsupported trap {:?}", riscv::register::scause::read().cause());
                        break;
                    }
                }

                if current_task.get_inner().status == TaskStatus::Zombie {
                    break;
                }
            }

            schedule::exit_current();
        }

        let pid_handle = pid_alloc();
        let mut pt = KERNEL_SPACE.lock().get_page_table().spawn(pid_handle.value);

        let mut kernel_stack = KernelStack::new();
        let kernel_stack_top = kernel_stack.area.vpn_range.get_end().0 << 12;
        let mut user_ctx = UserContext::default();
        let mut task_ctx = TaskContext::default();
        kernel_stack.area.map(&mut pt);

        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_sp, entry_point) = MemorySet::from_elf(elf_data, pt);

        task_ctx.set_instruction_pointer(task_kernel_entry as usize);
        task_ctx.set_stack_pointer(kernel_stack_top);

        user_ctx.set_ip(entry_point);
        user_ctx.set_sp(user_sp);

        info!("create task pid {} entry: {:#x}", pid_handle.value, entry_point);
        
        let task = Self {
            pid: pid_handle,
            kstack: kernel_stack,
            inner: ForceSync::new(UnsafeCell::new(TaskInner{
                memory_set,
                task_ctx: task_ctx,
                user_ctx: user_ctx,
                parent: None,
                children: [].to_vec(),
                exit_code: 0,
                fd_table: [].to_vec(),
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
        unsafe {self.as_mut_ptr().as_mut().unwrap()}
    }

    pub fn get_inner(&self) -> &TaskInner {
        unsafe {self.as_mut_ptr().as_ref().unwrap()}
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
        Task::new(v.as_slice())
    });
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}
