use core::cell::UnsafeCell;

extern crate spin;

#[macro_export]
macro_rules! cpu_local {
    ($vis:vis static ref $name:ident: $type:ty = $init:expr;) => {
        $vis struct $name {
            value: Once<$crate::cpu::local::CpuLocalCell<$type>>,
        }

        $vis static $name: $name = $name {
            value: spin::Once::INIT,
        };

        impl $name {
            pub fn as_mut_ptr(&self) -> *mut $type {
                fn __static_ref_init() -> $crate::cpu::local::CpuLocalCell<$type> {
                    $crate::cpu::local::CpuLocalCell::__new($init)
                }

                self.value.call_once(|| __static_ref_init()).get_mut()
            }

            pub fn as_mut(&self) -> &mut $type {
                unsafe { &mut *self.as_mut_ptr() }
            }
        }
    };
}

pub struct CpuLocalCell<T>(UnsafeCell<T>);
unsafe impl<T> Sync for CpuLocalCell<T> {}

impl<T> CpuLocalCell<T> {
    pub fn __new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }

    pub fn get_mut(&self) -> *mut T {
        self.0.get()
    }
}
