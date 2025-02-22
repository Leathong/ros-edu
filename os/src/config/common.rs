// pub const CONS_1K: usize = 0x400;
pub const CONS_4K: usize = 0x1000;
pub const CONS_1M: usize = 0x0010_0000;
// pub const CONS_1G: usize = 0x4000_0000;

pub const USER_STACK_SIZE: usize = CONS_4K * 2;
pub const KERNEL_STACK_SIZE: usize = CONS_4K * 2;
pub const KERNEL_HEAP_SIZE: usize = 3 * CONS_1M;
pub const PAGE_SIZE: usize = CONS_4K;
pub const PAGE_SIZE_BITS: usize = 12;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;
/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}
