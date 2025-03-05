use core::arch::asm;

use crate::mm::address::PhysPageNum;
use crate::mm::{address::VirtPageNum, frame_allocator::frame_alloc};
use alloc::vec::Vec;
use bitflags::bitflags;
use riscv::register::satp;

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
        PageTableEntry { bits: 0 }
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

const TEMP_PAGE_ADDR: usize = (0o777_777_775 << 12) | !(1 << 39 - 1);
const ROOT_PAGE_ADDR: usize = (0o777_777_776 << 12) | !(1 << 39 - 1);
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
        entries[510] =
            PageTableEntry::new(kernel_pt.root_ppn, PTEFlags::V | PTEFlags::R | PTEFlags::W);
        entries[511] = PageTableEntry::new(kernel_pt.root_ppn, PTEFlags::V);

        let skernel = VirtAddr::from(linker_args::skernel as usize);
        let spage = VirtPageNum::from(skernel);
        let idx = spage.indexes();
        // Assume the maximum memory is 1G, so only create 1 level-1 PTD.
        entries[idx[0]] = PageTableEntry::new(l1ppn, PTEFlags::V);

        let kstack_ppn = frame_alloc().unwrap();
        entries[508] = PageTableEntry::new(kstack_ppn, PTEFlags::V);
        kernel_pt
    }

    pub fn spawn(&self) -> Self {
        debug_assert!(self.kernel_started);
        let pt = PageTable {
            root_ppn: frame_alloc().unwrap(),
            kernel_started: true,
        };

        // copy kernel space to the new page table
        let root_entries = self.root_pte_array();
        self.map_temp_page(pt.root_ppn);
        let entries =
            unsafe { core::slice::from_raw_parts_mut(ROOT_PAGE_ADDR as *mut PageTableEntry, 512) };
        entries[256..512].copy_from_slice(&root_entries[256..512]);
        pt
    }

    fn root_pte_array(&self) -> &mut [PageTableEntry] {
        unsafe { core::slice::from_raw_parts_mut(ROOT_PAGE_ADDR as *mut PageTableEntry, 512) }
    }

    fn map_temp_page(&self, ppn: PhysPageNum) {
        let root_entries = self.root_pte_array();
        root_entries[509] = PageTableEntry::new(ppn, PTEFlags::V | PTEFlags::R | PTEFlags::W);
        unsafe {
            asm!(
                "li t0, {addr}", 
                "sfence.vma t0, x0", 
                addr = const TEMP_PAGE_ADDR);
        }
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

    pub fn active(&mut self) {
        let satp = self.token();
        self.kernel_started = true;
        unsafe {
            satp::write(satp::Satp::from_bits(satp));
            asm!("sfence.vma");
        }
    }
}

///Array of u8 slice that user communicate with os
pub struct UserBuffer {
    ///U8 vec
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    ///Create a `UserBuffer` by parameter
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    ///Length of `UserBuffer`
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}
/// Iterator of `UserBuffer`
pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
