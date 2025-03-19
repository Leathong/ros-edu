use crate::sbi;

pub fn sys_shutdown() -> ! {
    sbi::shutdown(false);
}
