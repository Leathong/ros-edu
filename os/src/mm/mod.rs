mod address;
mod page_table;
mod frame_allocator;
mod heap_allocator;
mod memory_set;
mod linker_args;

pub use memory_set::KERNEL_SPACE;

use linker_args::{sbss, ebss};

pub fn init() {
    heap_allocator::init_heap();
    frame_allocator::init_frame_alocator();
    // KERNEL_SPACE.lock().activate();
}

pub fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}