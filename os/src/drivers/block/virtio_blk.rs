use core::{alloc::Layout, ptr::NonNull};

use crate::{
    config::{KERNEL_SPACE_OFFSET, PAGE_SIZE},
    mm::{
        address::{PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum},
        frame_allocator::{frame_alloc, frame_dealloc},
        heap_allocator,
        memory_set::KERNEL_SPACE,
    },
    println,
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
                    println!(
                        "Detected virtio MMIO device with vendor id {:#X}, device type {:?}, version {:?}",
                        transport.vendor_id(),
                        transport.device_type(),
                        transport.version(),
                    );
                    let mut blk = VirtIOBlk::new(transport).expect("failed to create blk driver");
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
        // .expect("Error when reading VirtIOBlk");
    }
    fn write_block(&self, block_id: usize, buf: &[u8]) {
        self.virtio_blk
            .lock()
            .write_blocks(block_id, buf)
            .expect("Error when writing VirtIOBlk");
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
        let ptr = heap_allocator::KERNEL_HEAP_ALLOCATOR
            .lock()
            .alloc(Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap())
            .unwrap();
        let vaddr = VirtAddr::from(ptr.as_ptr() as usize);
        println!("{}::{} ptr {:x}", file!(), line!(), vaddr.0);
        let pte = KERNEL_SPACE
            .lock()
            .get_page_table()
            .translate(VirtPageNum::from(vaddr))
            .unwrap();
        println!("{}::{}", file!(), line!());
        unsafe {
            ADDR_OFF_SET = ptr.as_ptr() as usize - pte.ppn().0 << 12;
        }
        println!("{}::{}", file!(), line!());
        println!(
            "dma_alloc: {:x} {:x} {:x}",
            pte.ppn().0,
            ptr.as_ptr() as usize,
            unsafe { ADDR_OFF_SET }
        );
        (pte.ppn().0, ptr)
    }

    unsafe fn dma_dealloc(
        paddr: virtio_drivers::PhysAddr,
        vaddr: NonNull<u8>,
        pages: usize,
    ) -> i32 {
        heap_allocator::KERNEL_HEAP_ALLOCATOR.lock().dealloc(
            vaddr,
            Layout::from_size_align(pages * PAGE_SIZE, PAGE_SIZE).unwrap(),
        );
        0
    }

    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, size: usize) -> NonNull<u8> {
        if paddr < KERNEL_SPACE_OFFSET {
            NonNull::new(paddr as *mut u8).unwrap()
        } else {
            NonNull::new((paddr + unsafe { ADDR_OFF_SET }) as *mut u8).unwrap()
        }
    }

    unsafe fn share(
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) -> virtio_drivers::PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as usize;
        if vaddr < KERNEL_SPACE_OFFSET {
            vaddr
        } else {
            vaddr - unsafe { ADDR_OFF_SET }
        }
    }

    unsafe fn unshare(
        paddr: virtio_drivers::PhysAddr,
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) {
        // do nothing
    }
}
