use core::arch::{asm, global_asm};

use crate::mm::address::PhysPageNum;
use crate::mm::{address::VirtPageNum, frame_allocator::frame_alloc};
use crate::println;
use alloc::vec::Vec;
use bitflags::bitflags;
use riscv::register::satp;

use super::address::VirtAddr;
use super::linker_args;

global_asm!(include_str!("switch_env.S"));

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
    asid: usize,
}

unsafe extern "C" {
    fn _ptenv_switch();
    fn _ptenv_restore();
}

const TEMP_PAGE_ADDR: usize = (0o777_777_775 << 12) | !(1 << 39 - 1);
const ROOT_PAGE_ADDR: usize = (0o777_777_776 << 12) | !(1 << 39 - 1);
/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new_kernel() -> Self {
        let kernel_pt = PageTable {
            root_ppn: frame_alloc().unwrap(),
            asid: 1,
        };
        let entries = kernel_pt.root_ppn.get_pte_array();
        kernel_pt.root_ppn.get_bytes_array().fill(0);

        kernel_pt
    }

    pub fn spawn(&self, asid: usize) -> Self {
        let pt = PageTable {
            root_ppn: frame_alloc().unwrap(),
            asid: asid,
        };

        // copy kernel space to the new page table
        let root_entries = self.root_pte_array();
        self.map_temp_page(pt.root_ppn);
        let entries =
            unsafe { core::slice::from_raw_parts_mut(ROOT_PAGE_ADDR as *mut PageTableEntry, 512) };
        entries.fill(PageTableEntry::empty());
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

    fn find_pte(root_ppn: PhysPageNum, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        Self::find_or_crate_pte(root_ppn, vpn, false)
    }

    fn find_pte_create(root_ppn: PhysPageNum, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        Self::find_or_crate_pte(root_ppn, vpn, true)
    }

    fn find_or_crate_pte(root_ppn: PhysPageNum, vpn: VirtPageNum, create: bool) -> Option<&'static mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let ptes = ppn.get_pte_array();
            let pte = ptes.get_mut(*idx).unwrap();
            if i == 2 {
                result = Some(pte);
                break;
            }
            // println!("\t\tlevel {} pte {:#x} vpn {:#x}", i, pte.bits, vpn.0);
            if !pte.is_valid() {
                if create {
                    // println!("-----create\t{:x}\t{}\t{:x}", i, *idx, vpn.0);
                    let frame = frame_alloc().unwrap();
                    frame.get_bytes_array().fill(0);
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
        let mut root_ppn = self.root_ppn.0;
        unsafe {
            asm!("mv t0, {root_ppn}", "mv {root_ppn}, t0", root_ppn = inout(reg) root_ppn);
            _ptenv_switch();
        }
        // println!("lalalalal");
        Self::map_internal(PhysPageNum::from(root_ppn), vpn, ppn, flags);
        unsafe {
            _ptenv_restore();
        }
    }

    fn map_internal(root_ppn: PhysPageNum, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = Self::find_pte_create(root_ppn, vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
        // println!("--- map {:x} to {:x}", vpn.0, ppn.0)
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        let mut root_ppn = self.root_ppn.0;
        unsafe {
            asm!("mv t0, {root_ppn}", "mv {root_ppn}, t0", root_ppn = inout(reg) root_ppn);
            _ptenv_switch();
        }
        Self::unmap_internal(PhysPageNum::from(root_ppn), vpn);
        unsafe {
            _ptenv_restore();
        }
    }

    fn unmap_internal(root_ppn: PhysPageNum, vpn: VirtPageNum) {
        let pte = Self::find_pte(root_ppn, vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        let mut root_ppn = self.root_ppn.0;
        unsafe {
            asm!("mv t0, {root_ppn}", "mv {root_ppn}, t0", root_ppn = inout(reg) root_ppn);
            _ptenv_switch();
        }
        let res = Self::find_pte(PhysPageNum::from(root_ppn), vpn).map(|pte| *pte);
        unsafe {
            _ptenv_restore();
        }
        res
    }
    pub fn token(&self) -> usize {
        assert!(self.asid < 1 << 16, "asid overflow {:#x}", self.asid);
        0b1000usize << 60 | self.root_ppn.0 | self.asid << 44
    }

    pub fn active(&mut self) {
        let satp = self.token();
        unsafe {
            satp::write(satp::Satp::from_bits(satp));
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
