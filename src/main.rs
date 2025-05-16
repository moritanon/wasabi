#![no_main]
#![no_std]
#![feature(offset_of)]

use core::panic::PanicInfo;
use core::time::Duration;

use wasabi::init::init_basic_runtime;
use wasabi::init::init_display;
use wasabi::init::init_paging;
use wasabi::init::init_pci;
use wasabi::qemu::exit_qemu;
use wasabi::qemu::QemuExitCode;
use wasabi::println;
use wasabi::uefi::EfiHandle;
use wasabi::uefi::init_vram;
use wasabi::uefi::EfiSystemTable;
use wasabi::uefi::locate_loaded_image_protocol;
use wasabi::serial::SerialPort;

use wasabi::allocator::init_allocator;
use wasabi::print::set_global_vram;

use wasabi::info;
use wasabi::warn;
use wasabi::error;
use wasabi::x86::init_exception;
use wasabi::hpet::init_hpet;
use wasabi::hpet::global_timestamp;
use wasabi::executor::sleep;
use wasabi::executor::spawn_global;
use wasabi::executor::start_global_executor;

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

    init_display(&mut vram);

    set_global_vram(vram);
    //let mut w = BitmapTextWriter::new(vram);
    for i in 0..4{
        println!("i={i}");
    } 
    let acpi = efi_system_table.acpi_table().expect("ACPT table not found");

    let memory_map = init_basic_runtime(image_handle, efi_system_table);

    init_allocator(&memory_map);

    println!("Hello, Non-UEFI world!");
    let cr3 = wasabi::x86::read_cr3();
    println!("cr3 = {cr3:#p}");
    let t = Some(unsafe{&*cr3});
    println!("{t:?}");
    
    let (_gdt, _idt) = init_exception();
    info!("Exception initialized.");

    init_paging(&memory_map);
    info!("Now we are using our own Page Tables!");

    init_hpet(acpi);

    init_pci(acpi);

    let t0 = global_timestamp();

    let task1= async move {
        for i in 100..103 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            //yield_execution().await;
            sleep(Duration::from_secs(1)).await;
        }
        Ok(())
    };

    let task2 = async move {
        for i in 200..203 {
            info!("{i} hpet.main_counter = {:?}", global_timestamp() - t0);
            sleep(Duration::from_secs(2)).await;
        }
        Ok(())
    };

    let serial_task = async {
        let sp = SerialPort::default();
        if let Err(e) = sp.loopback_test() {
            error!("{e:?}");
            return Err("serial: loopback test failed");
        }
        info!("Started to monitor serial port");
        loop {
            if let Some(v) = sp.try_read() {
                let c = char::from_u32(v as u32);
                info!("serial input: {v:#04X} = {c:?}");
            }
            sleep(Duration::from_millis(20)).await;
        }
    };
    spawn_global(task1);
    spawn_global(task2);
    spawn_global(serial_task);
    start_global_executor();

}



//use core::{panic::PanicInfo, slice};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    exit_qemu(QemuExitCode::Fail)
}

