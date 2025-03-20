// SPDX-License-Identifier: MPL-2.0

//! The architecture support of context switch.

pub(crate) mod context;
pub(crate) mod schedule;
mod stack;
mod taskid;
mod utils;

use core::cell::UnsafeCell;

use crate::{
    cpu::{local::CpuLocalCell, processor::PROCESSOR},
    fs::{Stdin, Stdout},
    sync::mutex::Lock,
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

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use alloc::{
    sync::{Arc, Weak},
    vec,
};

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
    Waiting,
}

pub struct Task {
    pub taskid: TaskId,
    pub tid: usize,

    inner: ForceSync<UnsafeCell<TaskInner>>,
}

impl Task {
    pub fn new_with_elf(elf_data: &[u8]) -> Arc<Self> {
        let kernel_space = KERNEL_SPACE.lock();
        let taskid = taskid_alloc();
        let pt = kernel_space.get_page_table().spawn(taskid.value);
        let current_task = current_task();
        let mut cur_pt = kernel_space.get_page_table();

        let task: Arc<Task>;
        if let Some(cur) = current_task {
            task = cur.clone();
            cur_pt = task
                .get_mutable_inner()
                .memory_set
                .get_mut()
                .get_page_table();
        }

        // memory_set with elf
        let (memory_set, entry_point) = MemorySet::from_elf(elf_data, pt, cur_pt);
        let task = Task::new(
            Arc::new(CpuLocalCell::new(memory_set)),
            taskid,
            0,
            entry_point,
        );
        task.get_mutable_inner().process = Arc::downgrade(&task);
        task
    }

    pub fn new(
        memory_set: Arc<CpuLocalCell<MemorySet>>,
        taskid: TaskId,
        tid: usize,
        entry_point: usize,
    ) -> Arc<Self> {
        extern "C" fn task_kernel_entry() {
            trace!("task_kernel_entry");
            loop {
                let current_task = current_task().unwrap();
                let inner = current_task.get_mutable_inner();
                trace!("run task {}", current_task.taskid.value);
                inner.status = TaskStatus::Running;
                inner.user_ctx.run();
                let cause = riscv::register::scause::read().cause();
                trace!(
                    "trap from task {} cause: {:?}",
                    current_task.taskid.value, cause
                );
                unsafe {
                    set_sum();
                }
                let syscall_res;
                match cause.try_into().unwrap() {
                    Trap::Interrupt(Interrupt::SupervisorTimer) => {
                        set_next_trigger();
                        drop(current_task);
                        schedule::yield_now();
                        unreachable!()
                    }
                    Trap::Exception(Exception::UserEnvCall) => {
                        inner.user_ctx.sepc += 4;
                        syscall_res = syscall::handle_syscall(
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

                let status = current_task.get_inner().status;
                if status == TaskStatus::Ready || status == TaskStatus::Waiting {
                    drop(current_task);
                    // this function does not return, manually drop all the variables on the stack
                    schedule::yield_now();
                    unreachable!()
                } else if status == TaskStatus::Zombie {
                    drop(current_task);
                    break;
                }

                inner.user_ctx.general.a0 = syscall_res;
            }

            // this function does not return, manually drop all the variables on the stack
            schedule::exit_current();
            unreachable!()
        }

        let kernel_stack = KernelStack::new(taskid.value);
        let kernel_stack_top = kernel_stack.area.vpn_range.get_end().0 << 12;
        let mut user_ctx = UserContext::default();
        let mut task_ctx = TaskContext::default();

        let user_stack = UserStack::new(tid);
        let user_stack_top = user_stack.area.vpn_range.get_end().0 << 12;

        memory_set.get_mut().push(kernel_stack.area, None);
        memory_set.get_mut().push(user_stack.area, None);

        task_ctx.set_instruction_pointer(task_kernel_entry as usize);
        task_ctx.set_stack_pointer(kernel_stack_top);

        user_ctx.set_ip(entry_point);
        user_ctx.set_sp(user_stack_top);

        debug!("create task id {}", taskid.value);

        let task = Arc::new(Self {
            taskid: taskid,
            tid: tid,
            inner: ForceSync::new(UnsafeCell::new(TaskInner {
                memory_set: memory_set,
                task_ctx: task_ctx,
                user_ctx: user_ctx,
                process: Weak::new(),
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
                mutex_list: [].to_vec(),
                threads: vec![None],
                waiting_tasks: VecDeque::new(),
                status: TaskStatus::Ready,
            })),
        });
        task
    }

    pub fn new_thread(process: &Arc<Task>, entry_point: usize, arg: usize) -> Arc<Self> {
        let threadid = process.get_mutable_inner().alloc_thread();
        let taskid = taskid_alloc();
        let thread = Task::new(
            process.get_inner().memory_set.clone(),
            taskid,
            threadid,
            entry_point,
        );
        process.get_mutable_inner().threads[threadid] = Some(thread.clone());
        thread.get_mutable_inner().user_ctx.general.a0 = arg;
        thread.get_mutable_inner().process = Arc::downgrade(process);
        thread
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
    pub memory_set: Arc<CpuLocalCell<MemorySet>>,
    pub task_ctx: TaskContext,
    pub user_ctx: UserContext,

    pub process: Weak<Task>,
    pub parent: Option<Weak<Task>>,
    pub children: Vec<Arc<Task>>,
    pub exit_code: i32,
    pub fd_table: Vec<Option<Arc<dyn File + Send + Sync>>>,
    pub mutex_list: Vec<Option<Arc<dyn Lock>>>,
    pub threads: Vec<Option<Arc<Task>>>,
    pub waiting_tasks: VecDeque<Arc<Task>>,
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

    pub fn alloc_thread(&mut self) -> usize {
        if let Some(tid) = (1..self.threads.len()).find(|tid| self.threads[*tid].is_none()) {
            tid
        } else {
            self.threads.push(None);
            self.threads.len() - 1
        }
    }

    pub fn alloc_mutex(&mut self) -> usize {
        if let Some(tid) = (0..self.mutex_list.len()).find(|tid| self.mutex_list[*tid].is_none()) {
            tid
        } else {
            self.mutex_list.push(None);
            self.mutex_list.len() - 1
        }
    }
}

unsafe extern "C" {
    pub(crate) fn context_switch(cur: *const TaskContext, nxt: *const TaskContext);
}

lazy_static! {
    ///Globle process that init user shell
    pub static ref INITPROC: Arc<Task> = {
        let inode = open_file("initproc", OpenFlags::RDONLY).unwrap();
        let v = inode.read_all();
        Task::new_with_elf(v.as_slice())
    };
}

pub fn add_initproc() {
    add_task(INITPROC.clone());
}

pub fn current_task() -> Option<Arc<Task>> {
    PROCESSOR.as_mut().current()
}

pub fn current_process() -> Option<Arc<Task>> {
    PROCESSOR
        .as_mut()
        .current()
        .unwrap()
        .get_inner()
        .process
        .upgrade()
}
