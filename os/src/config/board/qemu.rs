use crate::config::CONS_1M;

pub const CLOCK_FREQ: usize = 12500000;
pub const KERNEL_HEAP_SIZE: usize = 5 * CONS_1M;

pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_2000), // VIRT_TEST/RTC  in virt machine
    (0x1000_1000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_2000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_3000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_4000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_5000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_6000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_7000, 0x00_1000), // Virtio Block in virt machine
    (0x1000_8000, 0x00_1000), // Virtio Block in virt machine
];
