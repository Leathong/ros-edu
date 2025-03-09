use core::{alloc::Layout, ptr::NonNull};

use crate::{
    config::{KERNEL_SPACE_OFFSET, PAGE_SIZE},
    mm::{
        address::{VirtAddr, VirtPageNum},
        heap_allocator,
        linker_args::entry_end_addr,
        memory_set::KERNEL_SPACE,
    },
};
use easy_fs::BlockDevice;
use spin::Mutex;
use virtio_drivers::{
    Hal,
    device::blk::VirtIOBlk,
    transport::{
        Transport,
        mmio::{MmioTransport, VirtIOHeader},
    },
};

use log::info;

#[allow(unused)]
const VIRTIO0: usize = 0x10001000;

pub struct VirtIOBlock {
    virtio_blk: Mutex<VirtIOBlk<VirtioHal, MmioTransport>>,
}

impl VirtIOBlock {
    #[allow(unused)]
    pub fn new() -> Self {
        unsafe {
            let header = NonNull::new(VIRTIO0 as *mut VirtIOHeader).unwrap();
            let size = 0x1000;
            match MmioTransport::new(header, size) {
                Err(e) => panic!("Error creating VirtIO MMIO transport: {}", e),
                Ok(transport) => {
                    info!(
                        "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                        transport.vendor_id(),
                        transport.device_type(),
                        transport.version(),
                    );
                    let mut blk = match VirtIOBlk::new(transport) {
                        Ok(blk) => blk,
                        Err(e) => {
                            panic!("Failed to create virtio blk: {}", e);
                        }
                    };
                    VirtIOBlock {
                        virtio_blk: Mutex::new(blk),
                    }
                }
            }
        }
    }
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

static mut ADDR_OFF_SET: usize = 0;
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
        let pte = KERNEL_SPACE.lock().get_page_table().translate(vpn).unwrap();
        unsafe {
            ADDR_OFF_SET = (ptr.as_ptr() as usize) - (pte.ppn().0 << 12);
        }

        unsafe {
            core::slice::from_raw_parts_mut(ptr.as_ptr(), size).fill(0);
        }
        info!(
            "::{} dma_alloc: {:x} {:x} {:x}",
            line!(),
            pte.ppn().0 << 12,
            ptr.as_ptr() as usize,
            unsafe { ADDR_OFF_SET }
        );
        (pte.ppn().0 << 12, ptr)
    }

    unsafe fn dma_dealloc(
        paddr: virtio_drivers::PhysAddr,
        vaddr: NonNull<u8>,
        pages: usize,
    ) -> i32 {
        info!("::{} {:#x} {:#x}", line!(), paddr, vaddr.as_ptr() as usize);
        heap_allocator::KERNEL_HEAP_ALLOCATOR.lock().dealloc(
            vaddr,
            Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap(),
        );
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, size: usize) -> NonNull<u8> {
        if paddr < unsafe { entry_end_addr } {
            info!("::{} mmio_phys_to_virt: {:#x}", line!(), paddr);
            NonNull::new(paddr as *mut u8).unwrap()
        } else {
            let vaddr = NonNull::new((paddr + unsafe { ADDR_OFF_SET }) as *mut u8).unwrap();
            info!(
                "::{} mmio_phys_to_virt {:#x}",
                line!(),
                vaddr.as_ptr() as usize
            );
            vaddr
        }
    }

    unsafe fn share(
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) -> virtio_drivers::PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        if vaddr < KERNEL_SPACE_OFFSET {
            info!("::{} {:#x}", line!(), vaddr);
            vaddr
        } else {
            let paddr = vaddr - unsafe { ADDR_OFF_SET };
            info!("::{} {:#x} {:#x}", line!(), vaddr, paddr);
            paddr
        }
    }

    unsafe fn unshare(
        paddr: virtio_drivers::PhysAddr,
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) {
        info!("::{}", line!());
    }
}
