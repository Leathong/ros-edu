//! Implementation of [`MapArea`] and [`MemorySet`].

use super::address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
use super::address::{StepByOne, VPNRange};
use super::frame_allocator::frame_alloc;
use super::linker_args::*;
use super::page_table::{PTEFlags, PageTable};
use crate::config::{self, KERNEL_SPACE_OFFSET, PAGE_SIZE, USER_STACK_SIZE};
use crate::config::MMIO;
use alloc::vec::Vec;
use bitflags::bitflags;
use lazy_static::*;
use crate::println;
use spin::Mutex;

lazy_static! {
    /// a memory set instance through lazy_static! managing kernel space
    pub static ref KERNEL_SPACE: Mutex<MemorySet> =
        Mutex::new(MemorySet::new_kernel());
}

/// memory set structure, controls virtual-memory space
pub struct MemorySet {
    page_table: PageTable,
    areas: Vec<MapArea>,
}

impl MemorySet {
    pub fn new(pt: PageTable) -> Self {
        Self {
            page_table: pt,
            areas: Vec::new(),
        }
    }

    pub fn get_page_table(&self) -> &PageTable {
        &self.page_table
    }

    fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
        map_area.map(&mut self.page_table);
        if let Some(data) = data {
            map_area.copy_data(&mut self.page_table, data);
        }
        self.areas.push(map_area);
    }
    /// Without kernel stacks.
    pub fn new_kernel() -> Self {
        let mut memory_set = Self::new(PageTable::new_kernel());

        // map kernel sections
        println!(".text\t[{:#x}, {:#x})", stext as usize, etext as usize);
        println!(".rodata\t[{:#x}, {:#x})", srodata as usize, erodata as usize);
        println!(".data\t[{:#x}, {:#x})", sdata as usize, edata as usize);
        println!(
            ".bss\t[{:#x}, {:#x})",
            sbss_with_stack as usize, ebss as usize
        );
        println!("mapping .text section");
        let mut text_area = MapArea::new(
            (stext as usize).into(),
            (etext as usize).into(),
            MapType::KernelOffset,
            MapPermission::R | MapPermission::X,
        );
        text_area.map(&mut memory_set.page_table);

        // allocator has not been initialized yet.
        // memory_set.push(
        //     MapArea::new(
        //         (stext as usize).into(),
        //         (etext as usize).into(),
        //         MapType::KernelOffset,
        //         MapPermission::R | MapPermission::X,
        //     ),
        //     None,
        // );
        println!("mapping .rodata section");
        let mut rodata_area = MapArea::new(
            (srodata as usize).into(),
            (erodata as usize).into(),
            MapType::KernelOffset,
            MapPermission::R,
        );
        rodata_area.map(&mut memory_set.page_table);
        println!("mapping .data section");
        let mut data_area = MapArea::new(
            (sdata as usize).into(),
            (edata as usize).into(),
            MapType::KernelOffset,
            MapPermission::R | MapPermission::W,
        );
        data_area.map(&mut memory_set.page_table);
        println!("mapping .bss section");
        let mut bss_area = MapArea::new(
            (sbss_with_stack as usize).into(),
            (ebss as usize).into(),
            MapType::KernelOffset,
            MapPermission::R | MapPermission::W,
        );
        bss_area.map(&mut memory_set.page_table);
        println!("mapping kernel heap area");
        let mut heap_area = MapArea::new(
            (ekernel as usize).into(),
            (ekernel as usize + config::KERNEL_HEAP_SIZE).into(),
            MapType::KernelOffset,
            MapPermission::R | MapPermission::W,
        );
        heap_area.map(&mut memory_set.page_table);
        println!("mapping memory-mapped registers");
        for pair in MMIO {
            let mut mmio_area = MapArea::new(
                (*pair).0.into(),
                ((*pair).0 + (*pair).1).into(),
                MapType::Identical,
                MapPermission::R | MapPermission::W,
            );
            mmio_area.map(&mut memory_set.page_table);
        }
        memory_set
    }

    /// Include sections in elf and trampoline and TrapContext and user stack,
    /// also returns user_sp and entry point.
    pub fn from_elf(elf_data: &[u8], new_pt: PageTable) -> (Self, usize, usize) {
        let mut memory_set = Self::new(new_pt);
        // map program headers of elf, with U flag
        let elf = xmas_elf::ElfFile::new(elf_data).unwrap();
        let elf_header = elf.header;
        let magic = elf_header.pt1.magic;
        assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
        let ph_count = elf_header.pt2.ph_count();
        let mut max_end_vpn = VirtPageNum(0);
        for i in 0..ph_count {
            let ph = elf.program_header(i).unwrap();
            if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
                let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
                let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
                let mut map_perm = MapPermission::U;
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    map_perm |= MapPermission::R;
                }
                if ph_flags.is_write() {
                    map_perm |= MapPermission::W;
                }
                if ph_flags.is_execute() {
                    map_perm |= MapPermission::X;
                }
                let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
                max_end_vpn = map_area.vpn_range.get_end();
                memory_set.push(
                    map_area,
                    Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
                );
            }
        }
        // map user stack with U flags
        let max_end_va: VirtAddr = max_end_vpn.into();
        let mut user_stack_bottom: usize = max_end_va.into();
        // guard page
        user_stack_bottom += PAGE_SIZE;
        let user_stack_top = user_stack_bottom + USER_STACK_SIZE;
        memory_set.push(
            MapArea::new(
                user_stack_bottom.into(),
                user_stack_top.into(),
                MapType::Framed,
                MapPermission::R | MapPermission::W | MapPermission::U,
            ),
            None,
        );

        (
            memory_set,
            user_stack_top,
            elf.header.pt2.entry_point() as usize,
        )
    }

    pub fn activate(&mut self) {
        self.page_table.active();
    }
}

/// map area structure, controls a contiguous piece of virtual memory
pub struct MapArea {
    pub vpn_range: VPNRange,
    pub map_type: MapType,
    pub map_perm: MapPermission,
}

impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            // data_frames: BTreeMap::new(),
            map_type,
            map_perm,
        }
    }

    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) -> PhysPageNum {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::KernelOffset => {
                let virt_addr = VirtAddr::from(vpn).0 - KERNEL_SPACE_OFFSET;
                ppn = PhysAddr::from(virt_addr).into();
            }
            MapType::Identical => {
                ppn = PhysPageNum(vpn.0);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame;
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits()).unwrap();
        page_table.map(vpn, ppn, pte_flags);
        ppn
    }
    #[allow(unused)]
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
    #[allow(unused)]
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut current_vpn = self.vpn_range.get_start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .get_bytes_array()[..src.len()];
            dst.copy_from_slice(src);
            start += PAGE_SIZE;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapType {
    KernelOffset,
    Identical,
    Framed,
}

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    #[derive(Clone, Copy)]
    pub struct MapPermission: u8 {
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
    }
}

#[allow(unused)]
pub fn remap_test() {
    let mut kernel_space = KERNEL_SPACE.lock();
    let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
    let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
    let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
    assert!(
        !kernel_space
            .page_table
            .translate(mid_text.floor())
            .unwrap()
            .writable(),
    );
    assert!(
        !kernel_space
            .page_table
            .translate(mid_rodata.floor())
            .unwrap()
            .writable(),
    );
    assert!(
        !kernel_space
            .page_table
            .translate(mid_data.floor())
            .unwrap()
            .executable(),
    );
    println!("remap_test passed!");
}
