use core::arch::{asm, global_asm};
use core::mem;

use crate::mm::address::PhysPageNum;
use crate::mm::{address::VirtPageNum, frame_allocator::frame_alloc};
use alloc::vec::Vec;
use bitflags::bitflags;
use log::trace;
use macros::ptenv_call;
use riscv::register::sstatus::{Sstatus, set_sum};
use riscv::register::{satp, sstatus};

use super::frame_allocator::frame_dealloc;

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

static mut PTENV_TOKEN: usize = 0;

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
#[repr(C)]
pub struct PageTable {
    root_ppn: PhysPageNum,
    asid: usize,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    pub fn new_kernel() -> Self {
        let kernel_pt = PageTable {
            root_ppn: frame_alloc().unwrap(),
            asid: 1,
        };
        unsafe {
            set_sum();
            sstatus::write(Sstatus::from_bits(0));
        }
        kernel_pt.root_ppn.get_bytes_array().fill(0);
        let entries = kernel_pt.root_ppn.get_pte_array();

        let ppn = frame_alloc().unwrap();
        ppn.get_bytes_array().fill(0);
        entries[511] = PageTableEntry::new(ppn, PTEFlags::V);

        let ppn = frame_alloc().unwrap();
        ppn.get_bytes_array().fill(0);
        entries[510] = PageTableEntry::new(ppn, PTEFlags::V);

        unsafe {
            unsafe extern "C" {
                static identical_map_pt_addr: usize;
            }

            PTENV_TOKEN = 0b1000usize << 60 | (identical_map_pt_addr >> 12);
        }

        kernel_pt
    }

    pub(super) fn init_ptenv(&self) {
        let ptenv_ppn = frame_alloc().unwrap();
        let ptenv_entries = ptenv_ppn.get_pte_array();
        for i in 0..256 {
            unsafe { ptenv_entries[i] = mem::transmute((i << 28) | 0b1111) }
        }
        let entries = self.root_ppn.get_pte_array();
        ptenv_entries[256..512].copy_from_slice(&entries[256..512]);
        unsafe {
            PTENV_TOKEN = 0b1000usize << 60 | ptenv_ppn.0;
        }
    }

    pub(super) fn release(&self) {
        ptenv_call!(Self::release_internal, self)
    }

    fn release_internal(&self) {
        let root_entries = self.root_ppn.get_pte_array();
        for root in root_entries[2..256].into_iter() {
            if root.is_valid() {
                let ppn = root.ppn();
                let entries = ppn.get_pte_array();
                for entry in entries {
                    if entry.is_valid() {
                        let ppn = entry.ppn();
                        frame_dealloc(ppn);
                    }
                }
                frame_dealloc(ppn);
            }
        }
        frame_dealloc(self.root_ppn);
    }

    pub fn spawn(&self, asid: usize) -> Self {
        let mut pt = PageTable {
            root_ppn: 0.into(),
            asid: 0,
        };
        trace!("self: {:p} asid: {}", self, asid);
        ptenv_call!(
            Self::spawn_internal,
            out = (pt.root_ppn.0),
            out = (pt.asid),
            self,
            asid
        );
        pt
    }

    fn spawn_internal(&self, asid: usize) -> Self {
        trace!("self: {:p} asid: {}", self, asid);
        let pt = PageTable {
            root_ppn: frame_alloc().unwrap(),
            asid: asid,
        };

        // copy kernel space to the new page table
        let root_entries = self.root_ppn.get_pte_array();
        let entries = pt.root_ppn.get_pte_array();
        entries.fill(PageTableEntry::empty());
        entries[256..512].copy_from_slice(&root_entries[256..512]);
        entries[..2].copy_from_slice(&root_entries[..2]);
        pt
    }

    fn find_pte(&self, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        self.find_or_crate_pte(vpn, false)
    }

    fn find_pte_create(&self, vpn: VirtPageNum) -> Option<&'static mut PageTableEntry> {
        self.find_or_crate_pte(vpn, true)
    }

    fn find_or_crate_pte(
        &self,
        vpn: VirtPageNum,
        create: bool,
    ) -> Option<&'static mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
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

    pub fn update_perm(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        // info!("self: {:p} vpn = {:?}, flags = {:?}", self, vpn, flags);
        ptenv_call!(Self::update_perm_internal, self, vpn.0, flags.bits());
    }

    fn update_perm_internal(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        // info!("self: {:p} vpn = {:?}, flags = {:?}\n", self, vpn, flags);
        let pte = self.find_pte(vpn).unwrap();
        *pte = PageTableEntry::new(pte.ppn(), flags | PTEFlags::V);
    }

    pub fn map(&self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        // info!(
        //     "self {:p} {:x} to {:x} asid: {}",
        //     self, vpn.0, ppn.0, self.asid
        // );
        ptenv_call!(Self::map_internal, self, vpn.0, ppn.0, flags.bits());
    }

    fn map_internal(&self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(!pte.is_valid(), "vpn {:?} is mapped before mapping", vpn);
        *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
        // info!(
        //     "self {:p} {:x} to {:x} asid: {}\n",
        //     self, vpn.0, ppn.0, self.asid
        // );
        unsafe {
            asm!(
                "sfence.vma {va}",
                va = in(reg) vpn.0 << 12,
            );
        };
    }

    pub fn unmap(&mut self, vpn: VirtPageNum) {
        // info!("self {:p} {:x} asid: {}", self, vpn.0, self.asid);
        ptenv_call!(Self::unmap_internal, self, vpn.0);
    }

    fn unmap_internal(&self, vpn: VirtPageNum) {
        let pte = self.find_pte(vpn).unwrap();
        assert!(pte.is_valid(), "vpn {:?} is invalid before unmapping", vpn);
        *pte = PageTableEntry::empty();
    }

    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        let pte: usize;
        // info!("self: {:p} vpn: {:x}", self, vpn.0);
        ptenv_call!(Self::translate_internal, out = pte, self, vpn.0);
        unsafe {
            let pte: PageTableEntry = mem::transmute(pte);
            // info!("pte: {:x}", pte.bits);
            // panic!("translate");
            Some(pte)
        }
    }

    fn translate_internal(&self, vpn: VirtPageNum) -> PageTableEntry {
        // info!("self: {:p} vpn: {:x} asid: {:x}", self, vpn.0, self.asid);
        let pte = self.find_pte(vpn).unwrap();
        // info!("pte: {:p} {:x}", pte, pte.bits);
        *pte
    }

    pub fn token(&self) -> usize {
        assert!(self.asid < 1 << 16, "asid overflow {:#x}", self.asid);
        0b1000usize << 60 | self.root_ppn.0 | self.asid << 44
    }

    pub fn activate(&self) {
        let satp = self.token();
        unsafe {
            satp::write(satp::Satp::from_bits(satp));
            asm!("sfence.vma",);
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
