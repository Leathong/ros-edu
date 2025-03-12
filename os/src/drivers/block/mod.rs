mod virtio_blk;

use core::{panic, ptr::NonNull};

use alloc::sync::Arc;

use easy_fs::BlockDevice;
use fdt::node::FdtNode;
use lazy_static::lazy_static;
use spin::Once;
use virtio_blk::init_blk;
use virtio_drivers::transport::{
    DeviceType, Transport,
    mmio::{MmioTransport, VirtIOHeader},
};

pub fn virtio_probe(node: FdtNode) {
    let mut reg = match node.reg() {
        Some(reg) => reg,
        None => panic!("virtio_blk: no reg"),
    };
    if let Some(reg) = reg.next() {
        let addr = reg.starting_address as usize;
        let size = reg.size.unwrap();
        // info!("walk dt addr={:#x}, size={:#x}", paddr, size);
        // info!(
        //     "Device tree node {}: {:?}",
        //     node.name,
        //     node.compatible().map(Compatible::first),
        // );

        let header = NonNull::new(addr as *mut VirtIOHeader).unwrap();
        unsafe {
            match MmioTransport::new(header, size) {
                Err(_) => {
                    // info!("Error creating VirtIO MMIO transport: {}", e);
                    return;
                }
                Ok(transport) => match transport.device_type() {
                    DeviceType::Block => {
                        init_blk(transport);
                    }
                    _ => return,
                },
            }
        }
    }
}

static BLOCK_DEVICE_INNER: Once<Arc<dyn BlockDevice>> = Once::new();

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = BLOCK_DEVICE_INNER.get().unwrap().clone();
}
