use core::ptr::NonNull;

use crate::{
    mm::{
        address::{PhysAddr, PhysPageNum, StepByOne},
        frame_allocator::{frame_alloc, frame_dealloc},
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
                    let mut blk = VirtIOBlk::new(transport)
                        .expect("failed to create blk driver");
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

unsafe impl Hal for VirtioHal {
    fn dma_alloc(
        pages: usize,
        direction: virtio_drivers::BufferDirection,
    ) -> (virtio_drivers::PhysAddr, NonNull<u8>) {
        todo!()
    }

    unsafe fn dma_dealloc(
        paddr: virtio_drivers::PhysAddr,
        vaddr: NonNull<u8>,
        pages: usize,
    ) -> i32 {
        todo!()
    }

    unsafe fn mmio_phys_to_virt(paddr: virtio_drivers::PhysAddr, size: usize) -> NonNull<u8> {
        todo!()
    }

    unsafe fn share(
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) -> virtio_drivers::PhysAddr {
        todo!()
    }

    unsafe fn unshare(
        paddr: virtio_drivers::PhysAddr,
        buffer: NonNull<[u8]>,
        direction: virtio_drivers::BufferDirection,
    ) {
        todo!()
    }
}
