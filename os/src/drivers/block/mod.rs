mod virtio_blk;

use alloc::sync::Arc;

use easy_fs::BlockDevice;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref BLOCK_DEVICE: Arc<dyn BlockDevice> = Arc::new(virtio_blk::VirtIOBlock::new());
}
