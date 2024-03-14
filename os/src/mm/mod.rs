mod address;
mod page_table;
mod frame_allocator;
mod heap_allocator;

pub fn init() {
    heap_allocator::init_heap();
    heap_allocator::heap_test();
}