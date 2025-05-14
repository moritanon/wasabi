extern crate alloc;

use crate::allocator::ALLOCATOR;
use crate::uefi::exit_from_efi_services;
use crate::uefi::EfiHandle;
use crate::uefi::EfiSystemTable;
use crate::uefi::EfiMemoryType::*;
use crate::uefi::MemoryMapHolder;
use crate::uefi::VramBufferInfo;
use crate::x86::write_cr3;
use crate::x86::PageAttr;
use crate::x86::PAGE_SIZE;
use crate::x86::PML4;
use crate::graphics::Bitmap;
use crate::graphics::draw_test_pattern;
use crate::graphics::fill_rect;

use alloc::boxed::Box;
use core::cmp::max;

pub fn init_basic_runtime (
    image_handle: EfiHandle,
    efi_system_table: &EfiSystemTable
) -> MemoryMapHolder
{
    let mut memory_map = MemoryMapHolder::new();
    exit_from_efi_services(
        image_handle,
        efi_system_table,
        &mut memory_map,
    );
    ALLOCATOR.init_with_mmap(&memory_map);
    memory_map
}

pub fn init_paging(memory_map: &MemoryMapHolder) {
    let mut table = PML4::new();
    let mut end_of_mem = 0x1_0000_0000u64;
    for e in memory_map.iter() {
        match e.memory_type() {
            CONVENTIONAL_MEMORY | LOADER_CORE | LOADER_DATA => {
                end_of_mem = max(
                    end_of_mem,
                    e.physical_start()
                        + e.number_of_page() * (PAGE_SIZE as u64),
                );
            }
            _ => (),
        }
    }
    table
        .create_mapping(0, end_of_mem, 0, PageAttr::ReadWriteKernel)
        .expect("Failed to create initial page mapping.");
    // Unmap page 0 to detect null ptr dereference.
    table
        .create_mapping(0, 4096, 0, PageAttr::NotPresend)
        .expect("Failed to unmap page 0.");
    unsafe {
        write_cr3(Box::into_raw(table));
    }



}

pub fn init_display(vram: &mut VramBufferInfo) {
    let vw = vram.width();
    let vh = vram.height();
    fill_rect(vram, 0x000000, 0, 0, vw, vh).expect("fill rect failed");

    draw_test_pattern(vram);
}