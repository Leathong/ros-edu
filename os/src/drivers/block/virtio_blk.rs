use core::{alloc::Layout, ptr::NonNull};

use crate::{
    config::PAGE_SIZE,
    drivers::block::BLOCK_DEVICE_INNER,
    mm::{
        address::{VirtAddr, VirtPageNum},
        heap_allocator,
        memory_set::KERNEL_SPACE,
    },
    task::Task,
};
use alloc::sync::Arc;
use easy_fs::BlockDevice;
use spin::Mutex;
use virtio_drivers::{
    Hal,
    device::blk::VirtIOBlk,
    transport::{Transport, mmio::MmioTransport},
};

use log::trace;

pub fn init_blk(transport: MmioTransport) {
    trace!(
        "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
        transport.vendor_id(),
        transport.device_type(),
        transport.version(),
    );

    BLOCK_DEVICE_INNER.call_once(|| match VirtIOBlk::new(transport) {
        Ok(blk) => Arc::new(VirtIOBlock {
            virtio_blk: Mutex::new(blk),
        }),
        Err(e) => {
            panic!("Failed to create virtio blk: {}", e);
        }
    });
}

pub struct VirtIOBlock {
    virtio_blk: Mutex<VirtIOBlk<VirtioHal, MmioTransport>>,
}

impl BlockDevice for VirtIOBlock {
    fn read_block(&self, block_id: usize, buf: &mut [u8]) {
        let res = match self.virtio_blk.lock().read_blocks(block_id, buf) {
            Ok(_) => (),
            Err(err) => panic!("Error when reading VirtIOBlk {:?}", err),
        };
        res
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        let res = match self.virtio_blk.lock().write_blocks(block_id, buf) {
            Ok(_) => (),
            Err(err) => panic!("Error when writing VirtIOBlk {:?}", err),
        };
        res
    }
}

pub struct VirtioHal;
#[allow(unused)]
unsafe impl Hal for VirtioHal {
    fn dma_alloc(
        pages: usize,
        direction: virtio_drivers::BufferDirection,
    ) -> (virtio_drivers::PhysAddr, NonNull<u8>) {
        let size = pages * PAGE_SIZE;
        let ptr = heap_allocator::KERNEL_HEAP_ALLOCATOR
            .lock()
            .alloc(Layout::from_size_align(size, PAGE_SIZE).unwrap())
            .unwrap();
        let vaddr = VirtAddr::from(ptr.as_ptr() as usize);
        let vpn = VirtPageNum::from(vaddr);
        let pte;
        if let Some(task) = Task::current_task() {
            pte = task
                .get_inner()
                .memory_set
                .get_page_table()
                .translate(VirtPageNum::from(vaddr));
        } else {
            pte = KERNEL_SPACE
                .lock()
                .get_page_table()
                .translate(VirtPageNum::from(vaddr));
        }
        let ppn = pte.unwrap().ppn();
        unsafe {
            core::slice::from_raw_parts_mut(ptr.as_ptr(), size).fill(0);
        }
        trace!("dma_alloc: {:x} {:x}", ppn.0 << 12, ptr.as_ptr() as usize,);
        (ppn.0 << 12, ptr)
    }

    unsafe fn dma_dealloc(
        paddr: virtio_drivers::PhysAddr,
        vaddr: NonNull<u8>,
        pages: usize,
    ) -> i32 {
        trace!("{:#x} {:#x}", paddr, vaddr.as_ptr() as usize);
        heap_allocator::KERNEL_HEAP_ALLOCATOR.lock().dealloc(
            vaddr,
            Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap(),
        );
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, size: usize) -> NonNull<u8> {
        trace!("mmio_phys_to_virt: {:#x}", paddr);
        NonNull::new(paddr as *mut u8).unwrap()
    }

    // mem is crated by driver in stack, transfrom the virt adrees to phys adress
    unsafe fn share(
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) -> virtio_drivers::PhysAddr {
        let vaddr_value = buffer.as_ptr() as *mut u8 as usize;
        let vaddr = VirtAddr::from(vaddr_value);

        let pte;
        if let Some(task) = Task::current_task() {
            pte = task
                .get_inner()
                .memory_set
                .get_page_table()
                .translate(VirtPageNum::from(vaddr));
        } else {
            pte = KERNEL_SPACE
                .lock()
                .get_page_table()
                .translate(VirtPageNum::from(vaddr));
        }
        let ppn = pte.unwrap().ppn();
        let paddr = (ppn.0 << 12) | (vaddr_value & ((1 << 12) - 1));
        trace!("share: {:#x} {:#x}", vaddr_value, paddr);
        paddr
    }

    unsafe fn unshare(
        paddr: virtio_drivers::PhysAddr,
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) {
    }
}
