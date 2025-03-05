use buddy_system_allocator::LockedHeap;
use crate::println;

#[global_allocator]
pub static KERNEL_HEAP_ALLOCATOR: LockedHeap<32> = LockedHeap::empty();

pub fn init_kernel_heap(start:usize, size: usize) {
    unsafe {
        KERNEL_HEAP_ALLOCATOR
            .lock()
            .init(start, size);
    }
}

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[allow(unused)]
pub fn heap_test() {
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    unsafe extern "C" {
        fn sbss();
        fn ebss();
    }

    let bss_range = sbss as usize..ebss as usize;

    let a = Box::new(5);
    assert_eq!(*a, 5);
    let a_ptr = &raw const *a as usize;
    assert!(bss_range.contains(&a_ptr));
    drop(a);
    let mut v: Vec<usize> = Vec::new();
    for i in 0..500 {
        v.push(i);
    }
    for i in 0..500 {
        assert_eq!(v[i], i);
    }
    let v_ptr = v.as_ptr() as usize;
    assert!(bss_range.contains(&v_ptr));
    drop(v);
    println!("heap_test passed!");
}
