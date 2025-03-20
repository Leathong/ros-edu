use crate::{
    config::{KERNEL_SPACE_OFFSET, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_SIZE},
    mm::memory_set::{MapArea, MapPermission, MapType},
};

pub struct KernelStack {
    pub area: MapArea,
}

impl KernelStack {
    pub fn new(pid: usize) -> Self {
        Self {
            area: MapArea::new(
                (usize::MAX - (KERNEL_STACK_SIZE + PAGE_SIZE) * pid - KERNEL_STACK_SIZE + 1).into(),
                (usize::MAX - (KERNEL_STACK_SIZE + PAGE_SIZE) * pid).into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
        }
    }
}

pub struct UserStack {
    pub area: MapArea,
}

const USER_SPACE_MAX: usize = usize::MAX - KERNEL_SPACE_OFFSET;
impl UserStack {
    pub fn new(tid: usize) -> Self {
        Self {
            area: MapArea::new(
                (USER_SPACE_MAX - (USER_STACK_SIZE + PAGE_SIZE) * tid - USER_STACK_SIZE + 1).into(),
                (USER_SPACE_MAX - (USER_STACK_SIZE + PAGE_SIZE) * tid).into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
        }
    }
}
