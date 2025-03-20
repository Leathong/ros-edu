use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug)]
pub struct TaskId {
    pub value: usize,
}

impl Drop for TaskId {
    fn drop(&mut self) {
        PID_ALLOCATOR.lock().dealloc(self.value);
    }
}

pub struct PidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl PidAllocator {
    ///Create an empty `PidAllocator`
    pub fn new() -> Self {
        PidAllocator {
            current: 2,
            recycled: Vec::new(),
        }
    }
    ///Allocate a pid
    pub fn alloc(&mut self) -> TaskId {
        if let Some(pid) = self.recycled.pop() {
            TaskId { value: pid }
        } else {
            self.current += 1;
            TaskId {
                value: self.current - 1,
            }
        }
    }
    ///Recycle a pid
    pub fn dealloc(&mut self, pid: usize) {
        assert!(pid < self.current);
        assert!(
            !self.recycled.iter().any(|ppid| *ppid == pid),
            "pid {} has been deallocated!",
            pid
        );
        self.recycled.push(pid);
    }
}

lazy_static! {
    pub static ref PID_ALLOCATOR: Mutex<PidAllocator> = Mutex::new(PidAllocator::new());
}

///Allocate a pid from PID_ALLOCATOR
pub fn taskid_alloc() -> TaskId {
    PID_ALLOCATOR.lock().alloc()
}
