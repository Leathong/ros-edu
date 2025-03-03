use crate::{
    config::{KERNEL_SPACE_MAX, KERNEL_STACK_SIZE, PAGE_SIZE, USER_STACK_SIZE},
    mm::memory_set::{MapArea, MapPermission, MapType},
};

pub struct KernelStack {
    pub area: MapArea,
    pub has_guard_page: bool,
}

impl KernelStack {
    fn new() -> Self {
        Self {
            area: MapArea::new(
                (Self::get_kernel_stack_end() + PAGE_SIZE).into(),
                Self::get_kernel_stack_bottom().into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            has_guard_page: true,
        }
    }
    fn get_kernel_stack_end() -> usize {
        Self::get_kernel_stack_bottom() - KERNEL_STACK_SIZE + 1
    }

    fn get_kernel_stack_bottom() -> usize {
        KERNEL_SPACE_MAX
    }
}

pub struct UserStack {
    pub area: MapArea,
    pub has_guard_page: bool,
}
impl UserStack {
    fn new() -> Self {
        Self {
            area: MapArea::new(
                (Self::get_user_stack_end() + PAGE_SIZE).into(),
                Self::get_user_stack_bottom().into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W,
            ),
            has_guard_page: false,
        }
    }
    fn get_user_stack_end() -> usize {
        Self::get_user_stack_bottom() - USER_STACK_SIZE + 1
    }

    fn get_user_stack_bottom() -> usize {
        KernelStack::get_kernel_stack_end() - 1
    }
}
