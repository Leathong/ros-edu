use super::address::PhysPageNum;
use alloc::vec::Vec;
use lazy_static::*;
use spin::mutex::Mutex;

use crate::mm::address::PhysAddr;

type FrameAllocatorImpl = StackFrameAllocator;
lazy_static! {
    pub static ref FRAME_ALLOCATOR: Mutex<FrameAllocatorImpl> =
        Mutex::new(FrameAllocatorImpl::new());
}

// === public interface ===
pub fn init_frame_alocator(start: PhysPageNum, end: PhysPageNum) {
    FRAME_ALLOCATOR
        .lock()
        .init(PhysAddr::from(start).ceil(), PhysAddr::from(end).floor());
}

pub fn frame_alloc() -> Option<PhysPageNum> {
    FRAME_ALLOCATOR.lock().alloc()
}

pub fn frame_dealloc(ppn: PhysPageNum) {
    FRAME_ALLOCATOR.lock().dealloc(ppn);
}

// === test ===

// pub fn frame_allocator_test() {
//     let mut v: Vec<FrameTracker> = Vec::new();
//     for i in 0..5 {
//         let frame = frame_alloc().unwrap();
//         println!("{:?}", frame);
//         v.push(frame);
//     }
//     v.clear();
//     for i in 0..5 {
//         let frame = frame_alloc().unwrap();
//         println!("{:?}", frame);
//         v.push(frame);
//     }
//     drop(v);
//     println!("frame_allocator_test passed!");
// }

// === impl ===

#[allow(unused)]
trait FrameAllocator {
    fn new() -> Self;
    fn alloc(&mut self) -> Option<PhysPageNum>;
    fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

impl StackFrameAllocator {
    pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
        self.current = l.0;
        self.end = r.0;
    }
}

impl FrameAllocator for StackFrameAllocator {
    fn new() -> Self {
        Self {
            current: 0,
            end: 0,
            recycled: Vec::new(),
        }
    }

    fn alloc(&mut self) -> Option<PhysPageNum> {
        if let Some(ppn) = self.recycled.pop() {
            Some(ppn.into())
        } else {
            if self.current == self.end {
                None
            } else {
                let res: PhysPageNum = self.current.into();
                self.current += 1;
                Some(res)
            }
        }
    }
    fn dealloc(&mut self, ppn: PhysPageNum) {
        let ppn = ppn.0;
        // validity check
        if ppn >= self.current || self.recycled.iter().find(|&v| *v == ppn).is_some() {
            panic!("Frame ppn={:#x} has not been allocated!", ppn);
        }
        // recycle
        self.recycled.push(ppn);
    }
}