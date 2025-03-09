pub mod block;

use alloc::vec;
pub use block::BLOCK_DEVICE;


#[allow(unused)]
pub fn virtio_blk_test() {
    let mut input = vec![0xffu8; 512];
    let mut output = vec![0; 512];
    for i in 0..32 {
        for x in input.iter_mut() {
            *x = i as u8;
        }
        BLOCK_DEVICE.write_block(i, &input);
        BLOCK_DEVICE.read_block(i, &mut output);
        assert_eq!(input, output);
    }
}