#![no_main]
#![no_std]
#![feature(offset_of)]

use core::fmt::Write;
use core::panic::PanicInfo;
use core::writeln;
use core::time::Duration;

use wasabi::graphics::Bitmap;
use wasabi::graphics::draw_test_pattern;
use wasabi::graphics::fill_rect;
use wasabi::init::init_basic_runtime;
use wasabi::init::init_paging;
use wasabi::qemu::exit_qemu;
use wasabi::qemu::QemuExitCode;
use wasabi::println;
use wasabi::uefi::EfiHandle;
use wasabi::uefi::init_vram;
use wasabi::uefi::EfiMemoryType;
use wasabi::uefi::EfiSystemTable;
use wasabi::uefi::VramTextWriter;
use wasabi::uefi::locate_loaded_image_protocol;

use wasabi::x86::read_cr3;
use wasabi::x86::PageAttr;
use wasabi::x86::flush_tlb;
use wasabi::info;
use wasabi::warn;
use wasabi::error;
use wasabi::hpet::Hpet;
use wasabi::executor::Task;
use wasabi::executor::Executor;
use wasabi::x86::init_exception;
use wasabi::hpet::global_timestamp;
use wasabi::hpet::set_global_hpet;
use wasabi::executor::TimeoutFuture;
//use wasabi::x86::trigger_debug_interrupt;
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

    let loaded_image_protocol = 
        locate_loaded_image_protocol(image_handle, efi_system_table)
        .expect("Failed to get LoadedImageProtocol.");
    println!("image_base: {:#018X}", loaded_image_protocol.image_base);
    println!("image_size: {:#018X}", loaded_image_protocol.image_size);

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
    let acpi = efi_system_table.acpi_table().expect("ACPT table not found");

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
    //trigger_debug_interrupt();
    //info!("Execution continued.");
    init_paging(&memory_map);
    info!("Now we are using our own Page Tables!");

    /* info!("reading from memory address 0... (first)");
    #[allow(clippy::zero_ptr)]
    #[allow(deref_nullptr)]
    let value_at_zero = unsafe { *(0 as *const u8) };
    let vref : &u8 = &value_at_zero;
    info!("value_at_zero = {value_at_zero}");
    info!("ref value_at_zero = {vref:?}"); */

    let page_table = read_cr3();
    unsafe {
        (*page_table)
            .create_mapping(0, 4096, 0, PageAttr::NotPresend)
            .expect("Failed to unmap page 0.");
    }
    flush_tlb();

    let hpet = acpi.hpet().expect("Filed to get HPET from ACPI");
    let hpet = hpet
        .base_address()
        .expect("Failed to get HPET base address");
    //info!("HPET is at {hpet:#018X}");
    let hpet = Hpet::new(hpet);
    set_global_hpet(hpet);
    let t0 = global_timestamp();
    /* info!("reading from memory address 0... (again)");
    #[allow(clippy::zero_ptr)]
    #[allow(deref_nullptr)]
    let value_at_zero = unsafe { *(0 as *const u8) };
    info!("value_at_zero = {value_at_zero} again"); 

    let result = block_on(async {
        info!("Hello from async world!");
        Ok(())
    });
    info!("block_on completed! result = {result:?}");
    */

    let task1= Task::new(async move {
        for i in 100..103 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            //yield_execution().await;
            TimeoutFuture::new(Duration::from_secs(1)).await;
        }
        Ok(())
    });

    let task2 = Task::new(async move {
        for i in 200..203 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            TimeoutFuture::new(Duration::from_secs(2)).await;
        }
        Ok(())
    });
    let mut executor = Executor::new();
    executor.enqueue(task1);
    executor.enqueue(task2);
    Executor::run(executor);

}



//use core::{panic::PanicInfo, slice};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit_qemu(QemuExitCode::Fail)
}

