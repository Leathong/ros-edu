use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;

#[derive(Debug)]
pub struct ProcessId {
    pub value: usize,
}

impl Drop for ProcessId {
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
    pub fn alloc(&mut self) -> ProcessId {
        if let Some(pid) = self.recycled.pop() {
            ProcessId {
                value: pid,
            }
        } else {
            self.current += 1;
            ProcessId {
                value: self.current - 1
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
pub fn pid_alloc() -> ProcessId {
    PID_ALLOCATOR.lock().alloc()
}

// pub struct ThreadId {
//     pub value: usize,
// }
