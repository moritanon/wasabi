#![no_main]
#![no_std]
#![feature(offset_of)]

use core::fmt::Write;
use core::panic::PanicInfo;
use core::writeln;

use wasabi::graphics::Bitmap;
use wasabi::graphics::draw_test_pattern;
use wasabi::graphics::fill_rect;
use wasabi::init::init_basic_runtime;
use wasabi::qemu::exit_qemu;
use wasabi::qemu::QemuExitCode;
use wasabi::println;
use wasabi::uefi::EfiHandle;
use wasabi::uefi::init_vram;
use wasabi::uefi::EfiMemoryType;
use wasabi::uefi::EfiSystemTable;
use wasabi::uefi::VramTextWriter;

use wasabi::x86::hlt;
use wasabi::info;
use wasabi::warn;
use wasabi::error;
use wasabi::x86::init_exception;
use wasabi::x86::trigger_debug_interrupt;
//use wasabi::print::hexdump;

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
    println!("Booting WasabiOS...");
    println!("image_handle: {:#018X}", image_handle);
    println!("efi_system_table: {:#p}", efi_system_table);

    info!("HOGEEEE");
    warn!("GUEEEEE");
    error!("GYAAAAAAAA");
  


    let mut vram = init_vram(efi_system_table).expect("init vram failed.");

    let vw = vram.width();
    let vh = vram.height();
    fill_rect(&mut vram, 0x000000, 0, 0, vw, vh).expect("fill rect failed");

    draw_test_pattern(&mut vram);

    let mut w = VramTextWriter::new(&mut vram);
    for i in 0..4{
        writeln!(w, "i={i}").unwrap();
    }

    let memory_map = init_basic_runtime(image_handle, efi_system_table);

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

    writeln!(w, "Hello, Non-UEFI world!").unwrap();
    let cr3 = wasabi::x86::read_cr3();
    println!("cr3 = {cr3:#p}");
    let t = Some(unsafe{&*cr3});
    println!("{t:?}");
    //let t = t.and_then(|t| t.next_level(0));
    //println!("{t:?}");
    //let t = t.and_then(|t| t.next_level(0));
    //println!("{t:?}");
    //let t = t.and_then(|t| t.next_level(0));
    //println!("{t:?}");
    
    let (_gdt, _idt) = init_exception();
    info!("Exception initialized.");
    trigger_debug_interrupt();

    loop {
        hlt()
    }
}



//use core::{panic::PanicInfo, slice};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit_qemu(QemuExitCode::Fail)
}

