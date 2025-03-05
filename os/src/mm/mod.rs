pub(crate) mod address;
pub(crate) mod frame_allocator;
pub(crate) mod heap_allocator;
pub(crate) mod page_table;
pub(crate) mod memory_set;
mod linker_args;

pub use page_table::UserBuffer;

use address::PhysAddr;
use memory_set::KERNEL_SPACE;
use crate::println;

use crate::config::{KERNEL_HEAP_SIZE, KERNEL_SPACE_OFFSET, PAGE_SIZE_BITS};

pub fn init(fdt: &fdt::Fdt) {
    clear_bss();
    let mem_reg = fdt.memory().regions().next().unwrap();
    let mem_start = mem_reg.starting_address as usize;
    let mem_size = mem_reg.size.unwrap_or(0);
    let mem_end = mem_start + mem_size;
    println!(
        "...memory range : start {:#X}, size {:#X}",
        mem_start, mem_size
    );

    let kernel_end: usize = linker_args::ekernel as usize;
    debug_assert!(kernel_end & (1 << PAGE_SIZE_BITS - 1) == 0);

    frame_allocator::init_frame_alocator(
        PhysAddr::from(kernel_end - KERNEL_SPACE_OFFSET).into(),
        PhysAddr::from(mem_end).floor(),
    );

    KERNEL_SPACE.lock().activate();
    heap_allocator::init_kernel_heap(kernel_end, KERNEL_HEAP_SIZE);
    println!("kernel memory initialized");
}

fn clear_bss() {
    unsafe {
        core::slice::from_raw_parts_mut(
            linker_args::sbss as usize as *mut u8,
            linker_args::ebss as usize - linker_args::sbss as usize,
        )
        .fill(0);
    }
}
