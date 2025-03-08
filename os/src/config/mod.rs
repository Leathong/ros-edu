pub mod board;

// pub const CONS_1K: usize = 0x400;
pub const CONS_4K: usize = 0x1000;
pub const CONS_1M: usize = 0x0010_0000;
// pub const CONS_1G: usize = 0x4000_0000;

pub const USER_STACK_SIZE: usize = CONS_4K * 16;
pub const KERNEL_STACK_SIZE: usize = CONS_4K * 16;
pub const PAGE_SIZE: usize = CONS_4K;
pub const PAGE_SIZE_BITS: usize = 12;

pub const KERNEL_SPACE_OFFSET: usize = 0xffff_ffc0_0000_0000;

pub use crate::config::board::qemu::*;
