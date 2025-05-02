#![no_main]
#![no_std]
#![feature(offset_of)]

use core::arch::asm;
use core::fmt::Write;
use core::panic::PanicInfo;
use core::writeln;

use wasabi::graphics::Bitmap;
use wasabi::graphics::draw_test_pattern;
use wasabi::graphics::fill_rect;

use wasabi::uefi::EfiHandle;
use wasabi::uefi::exit_from_efi_services;
use wasabi::uefi::init_vram;
use wasabi::uefi::EfiMemoryType;
use wasabi::uefi::EfiSystemTable;
use wasabi::uefi::MemoryMapHolder;
use wasabi::uefi::VramTextWriter;

pub fn hlt() {
    unsafe {
        asm!("hlt")
    }
}
#[no_mangle]
fn efi_main(image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    //let efi_graphics_output_protocol = locate_graphic_protocol(efi_system_table).unwrap();
    //let vram_addr = efi_graphics_output_protocol.mode.frame_buffer_base;
    //let vram_byte_size = efi_graphics_output_protocol.mode.frame_buffer_size;
    //let vram = unsafe {
    //    slice::from_raw_parts_mut(vram_addr as *mut u32, vram_byte_size / size_of::<u32>())
    //};
    //for e in vram {
    //    *e = 0xffffff;
    //}

    let mut vram = init_vram(efi_system_table).expect("init vram failed.");

    let vw = vram.width();
    let vh = vram.height();
    fill_rect(&mut vram, 0x000000, 0, 0, vw, vh).expect("fill rect failed");

    draw_test_pattern(&mut vram);

    let mut w = VramTextWriter::new(&mut vram);
    for i in 0..4{
        writeln!(w, "i={i}").unwrap();
    }

    let mut memory_map = MemoryMapHolder::new();
    let status = efi_system_table
                            .boot_services()
                            .get_memory_map(&mut memory_map);
    writeln!(w, "{status:?}").unwrap();
    let mut total_memory_pages = 0;
    for e in memory_map.iter() {
        if e.memory_type() != EfiMemoryType::CONVENTIONAL_MEMORY {
            continue;
        }
        total_memory_pages += e.number_of_page();
        writeln!(w, "{e:?}").unwrap();
    }
    let total_memory_size_mib = total_memory_pages * 4096 / 1024 /1024;
    writeln!(w, 
            "Total: {total_memory_pages} pages = {total_memory_size_mib}MiB"
        )
        .unwrap();

    exit_from_efi_services(
        image_handle,
        efi_system_table,
        &mut memory_map,
    );
    writeln!(w, "Hello, Non-UEFI world!").unwrap();
    loop {
        hlt()
    }
}



//use core::{panic::PanicInfo, slice};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {
        hlt();
    }
}

