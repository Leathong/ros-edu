extern crate alloc;
use crate::config::common::KERNEL_SPACE_OFFSET;
use crate::mm::address::PhysPageNum;
use crate::mm::{address::VirtPageNum, frame_allocator::frame_alloc};
use bitflags::bitflags;

use super::address::VirtAddr;
use super::linker_args;

bitflags! {
    /// page table entry flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PTEFlags: u8 {
        const V = 1 << 0; // valid
        const R = 1 << 1; // read
        const W = 1 << 2; // write
        const X = 1 << 3; // execute
        const U = 1 << 4; // user
        const G = 1 << 5;
        const A = 1 << 6; // accessed
        const D = 1 << 7; // dirty
    }
}


#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
    pub bits: usize,
}

impl PageTableEntry {
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits() as usize,
        }
    }
    pub fn empty() -> Self {
        PageTableEntry {
            bits: 0,
        }
    }
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
}

/// page table structure
pub struct PageTable {
    root_ppn: PhysPageNum,
    kernel_started: bool,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new_kernel() -> Self {
        let kernel_pt = PageTable {
            root_ppn: frame_alloc().unwrap(),
            kernel_started: false,
        };
        let entries = kernel_pt.root_ppn.get_pte_array();
        let l1ppn = frame_alloc().unwrap();

        // Place the root ppn to the last entry of the root PTD to achieve self-mapping.
        entries[510] = PageTableEntry::new(kernel_pt.root_ppn, PTEFlags::V | PTEFlags::R | PTEFlags::W);
        entries[511] = PageTableEntry::new(kernel_pt.root_ppn, PTEFlags::V);

        let skernel = VirtAddr::from(linker_args::skernel as usize);
        let spage = VirtPageNum::from(skernel);
        let idx = spage.indexes();
        // Assume the maximum memory is 1G, so only create 1 level-1 PTD.
        entries[idx[0]] = PageTableEntry::new(l1ppn, PTEFlags::V);
        kernel_pt
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        self.find_or_crate_pte(vpn, false)
    }

    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        self.find_or_crate_pte(vpn, true)
    }

    fn find_or_crate_pte(&self, vpn: VirtPageNum, create: bool) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = if self.kernel_started {
                &mut vpn.get_pte_array(*idx)[*idx]
            } else {
                &mut ppn.get_pte_array()[*idx]
            };
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                if create {
                    let frame = frame_alloc().unwrap();
                    *pte = PageTableEntry::new(frame, PTEFlags::V);
                } else {
                    return None;
                }
            }
            ppn = pte.ppn();
        }
        result
    }

    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    pub fn token(&self) -> usize {
        0b1000usize << 60 | self.root_ppn.0
    }
}

