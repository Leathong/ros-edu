use crate::{
    config::{KERNEL_SPACE_OFFSET, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_SIZE},
    mm::memory_set::{MapArea, MapPermission, MapType},
};

pub struct KernelStack {
    pub area: MapArea,
}

impl KernelStack {
    pub fn new() -> Self {
        Self {
            area: MapArea::new(
                (Self::get_kernel_stack_end() + PAGE_SIZE).into(),
                Self::get_kernel_stack_bottom().into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
        }
    }
    fn get_kernel_stack_end() -> usize {
        Self::get_kernel_stack_bottom() - KERNEL_STACK_SIZE + 1
    }

    // botton at high address
    fn get_kernel_stack_bottom() -> usize {
        usize::MAX - KERNEL_SPACE_OFFSET
    }
}

pub struct UserStack {
    pub area: MapArea,
}
impl UserStack {
    pub fn new() -> Self {
        Self {
            area: MapArea::new(
                (Self::get_user_stack_end() + PAGE_SIZE).into(),
                Self::get_user_stack_bottom().into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
        }
    }
    fn get_user_stack_end() -> usize {
        Self::get_user_stack_bottom() - USER_STACK_SIZE + 1
    }

    fn get_user_stack_bottom() -> usize {
        KernelStack::get_kernel_stack_end() - 1
    }
}
