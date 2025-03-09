mod virtio_blk;

use core::panic;

use alloc::sync::Arc;

use easy_fs::BlockDevice;
use fdt::{node::FdtNode, standard_nodes::Compatible};
use lazy_static::lazy_static;
use log::info;
use spin::Once;
use virtio_blk::init_blk;

pub fn virtio_probe(node: FdtNode) {
    let mut reg = match node.reg() {
        Some(reg) => reg,
        None => panic!("virtio_blk: no reg"),
    };
    if let Some(reg) = reg.next() {
        let paddr = reg.starting_address as usize;
        let size = reg.size.unwrap();
        info!("walk dt addr={:#x}, size={:#x}", paddr, size);
        info!(
            "Device tree node {}: {:?}",
            node.name,
            node.compatible().map(Compatible::first),
        );
        init_blk(paddr, size);
    }
}


static BLOCK_DEVICE_INNER: Once<Arc<dyn BlockDevice>> = Once::new();

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = BLOCK_DEVICE_INNER.get().unwrap().clone();
}
