extern crate alloc;

use crate::result::Result;
use crate::uefi::EfiMemoryDescriptor;
use crate::uefi::EfiMemoryType;
use crate::uefi::MemoryMapHolder;
use alloc::alloc::GlobalAlloc;
use alloc::alloc::Layout;
use alloc::boxed::Box;
use core::borrow::BorrowMut;
use core::cell::RefCell;
use core::cmp::max;
use core::fmt;
use core::mem::size_of;
use core::ops::DerefMut;
use core::ptr::null_mut;
use crate::serial::SerialPort;
use core::fmt::Write;

struct U8ptr(*mut u8);

/* impl U8Ptr {
    pub unsafe fn get_value(&self) -> Option<u8> {
        if self.is_null() {
            None
        } else {
            Some(*self.0)
        }
    }
}

impl fmt::Display for U8ptr {

    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.0.is_null() {
            write!(f, "null_ptr")
        } else {
            let value = unsafe {self.getValue()};
            write!(f, "{}", value)
        }
    }
} */

pub fn round_up_to_nearest_pow2(v: usize) -> Result<usize> {
    1usize
        .checked_shl(usize::BITS - v.wrapping_sub(1).leading_zeros())
        .ok_or("Out of range")
}

#[test_case]
fn round_up_to_nearest_pow2_tests() {
    //unimplemented!("cargo test should fail.. right..?")
    assert_eq!(round_up_to_nearest_pow2(0), Err("Out of range"));
    assert_eq!(round_up_to_nearest_pow2(100), Ok(128usize));
}

/// Vertical Bar '|' represents the chunk that has a Header 
/// before: |-- prev -------|---- self ---------------
/// align:  |--------|-------|-------|-------|-------|
/// after:  |---------------||-------|----------------

struct Header {
    next_header: Option<Box<Header>>,
    size: usize,
    is_allocated: bool,
    _reserved: usize,
}

const HEADER_SIZE:usize = size_of::<Header>();
#[allow(clippy::assertions_on_constants)]
const _: () = assert!(HEADER_SIZE == 32);
// Size of Header should bi power of 2
pub const LAYOUT_PAGE_4K: Layout =
    unsafe { Layout::from_size_align_unchecked(4096, 4096) };

impl Header {
    fn can_provide(&self, size: usize, align: usize) -> bool {
        // This check is rough - actual size needed may by smaller.
        // HEADR_SIZE  * 2 => one for allocated region, another for padding.
        self.size >= size + HEADER_SIZE * 2 + align
    }
    fn is_allocated(&self) -> bool {
        self.is_allocated
    }
    fn end_addr(&self) -> usize {
        self as *const Header as usize + self.size
    }
    unsafe fn new_from_addr(addr: usize) ->Box<Header> {
        let header = addr as *mut Header;
        header.write(Header {
            next_header: None,
            size: 0,
            is_allocated: false,
            _reserved:0,
        });
        Box::from_raw(addr as *mut Header)
    }
    unsafe fn from_allocated_region(addr: *mut u8) -> Box<Header> {
        let header = addr.sub(HEADER_SIZE) as *mut Header;
        Box::from_raw(header)
    }
    //
    // Note] std::alloc::Layout doc says:
    // > All layouts have an associated size and power-of-two alignment.
    fn provide(&mut self, size: usize, align: usize) -> Option<*mut u8> {
        let size = max(round_up_to_nearest_pow2(size).ok()?, HEADER_SIZE);
        let align = max(align, HEADER_SIZE);
        if self.is_allocated() || !self.can_provide(size, align) {
            None
        } else {
            // Each char represents 32-byte chunks
            //
            // |-----|-------------------self -----------|---------
            // |-----|-----------------------            |---------
            //                                           ^ self.end_addr()
            //                               |---------|-
            //                               ^ header_for_allocated
            //                                ^ allocated_addr
            //                                         ^ header_for_padding
            // header_for_allocated.end_addr() self has enough 
            // to allocate the requested object.

            // Make header for the allocated object 
            let mut size_used = 0;
            let allocated_addr = (self.end_addr() - size) & !(align - 1);
            let mut header_for_allocated = 
                unsafe { Self::new_from_addr(allocated_addr - HEADER_SIZE) };
            header_for_allocated.is_allocated = true;
            header_for_allocated.size = size + HEADER_SIZE;
            size_used += header_for_allocated.size;
            header_for_allocated.next_header = self.next_header.take();
            if header_for_allocated.end_addr() != self.end_addr() {
                // Make a header padding
                let mut header_for_padding = unsafe {
                    Self::new_from_addr(header_for_allocated.end_addr())
                };
                header_for_padding.is_allocated = false;
                header_for_padding.size =
                    self.end_addr() - header_for_allocated.end_addr();
                size_used += header_for_padding.size;
                header_for_padding.next_header = 
                    header_for_allocated.next_header.take();

                header_for_allocated.next_header = Some(header_for_padding);
            }
            // Shrink Self
            assert!(self.size >= size_used + HEADER_SIZE);
            self.size -= size_used;
            self.next_header = Some(header_for_allocated);
            Some(allocated_addr as *mut u8)
        }
    }
}

impl Drop for Header {
    fn drop(&mut self) {
        panic!("header should not be dropped!")
    }
}

impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Header @ {:#018X} {{ size {:#018X}({}k), is_allocated: {}, next: {:?}}}",
//            "Header @ {:#018X} {{ size {:#018X}({}k), is_allocated: {}}}",
            self as *const Header as usize,
            self.size,
            self.size/1024,
            self.is_allocated(),
            self.next_header,
        ) 
    }
}

pub struct FirstFitAllocator {
    first_header: RefCell<Option<Box<Header>>>,
}

#[global_allocator]
pub static ALLOCATOR: FirstFitAllocator = FirstFitAllocator {
    first_header: RefCell::new(None),
};

unsafe impl Sync for FirstFitAllocator {}

unsafe impl GlobalAlloc for FirstFitAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = self.alloc_with_options(layout);
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        let mut region  = Header::from_allocated_region(ptr);
        region.is_allocated = false;
        Box::leak(region);
        // region is leaked here to avoid dropping the free info on the memory.
    }
}

impl FirstFitAllocator {
    pub fn alloc_with_options(&self, layout: Layout) -> *mut u8 {
        let mut header = self.first_header.borrow_mut();
        let mut header = header.deref_mut();
//let mut sw = SerialPort::new_for_com1();
//writeln!(sw, "FirstFitAllocator alloc_with_options {:?}", header).unwrap(); 
        loop {
            match header {
                Some(e) => match e.provide(layout.size(), layout.align()) {
                    Some(p) => break p,
                    None => {
                        header = e.next_header.borrow_mut();
                        continue;
                    }
                },
                None => {
                    break null_mut::<u8>()
                }
            }
        }
    }
    pub fn init_with_mmap(&self, memory_map: &MemoryMapHolder) {
//let mut sw = SerialPort::new_for_com1();
        for e in memory_map.iter() {
            if e.memory_type() != EfiMemoryType::CONVENTIONAL_MEMORY {
                continue;
            }

            self.add_free_from_descriptor(e);
        }
//writeln!(sw, "init header by mmap {:?}", self.first_header).unwrap(); 
    }
    fn add_free_from_descriptor(&self, desc: &EfiMemoryDescriptor) {
        let mut start_addr = desc.physical_start() as usize;
        let mut size = desc.number_of_page() as usize * 4096;

        // Make sure the allocator does not include the address 0 as a free
        // area.
        if start_addr == 0 {
            start_addr += 4096;
            size = size.saturating_sub(4096);
        }
        if size <= 4096 {
            return;
        }
        let mut header = unsafe { Header::new_from_addr(start_addr)};
        header.next_header = None;
        header.is_allocated = false;
        header.size = size;
        let mut first_header = self.first_header.borrow_mut();
        let prev_last = first_header.replace(header);
        drop(first_header);
        let mut header = self.first_header.borrow_mut();
        header.as_mut().unwrap().next_header = prev_last;
//writeln!(sw, "init memory by mmap {:?}", header).unwrap(); 
        // It's okay  not to be sorted the headers at this point.
        // since all the regions written in memory maps are not contiguous
        // so that the can't be merged anyway 
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use alloc::vec;

    #[test_case]
    fn malloc_iterate_free_and_alloc() {
        use alloc::vec::Vec;
        for i in 0..1000 {
           let mut vec = Vec::new();
           vec.resize(i, 10);
           // vec will be deallocated at the end of this scope.
        }
    }

    #[test_case]
    fn malloc_align() {
        let mut pointers = [null_mut::<u8>(); 100];
        for align in [1, 2, 4, 8, 16, 32, 4096] {
            for e in pointers.iter_mut() {
                *e = ALLOCATOR.alloc_with_options(
                    Layout::from_size_align(1234, align)
                        .expect("Failed to create Layout"),
                );
                assert!(*e as usize != 0);
                assert!((*e as usize) % align == 0);
            }
        }
    }

    #[test_case]
    fn malloc_align_random_order() {
        //use crate::serial::SerialPort;
        //use core::fmt::Write;
        //let mut sw = SerialPort::new_for_com1();
        

//        for align in [32, 4096, 0, 4, 16, 2, 1] {
        for align in [32, 4096, 4, 16, 2, 1] {
                let mut pointers = [null_mut::<u8>(); 100];
            for e in pointers.iter_mut() {
                //writeln!(sw,"current align = {}", align).unwrap();
                *e = ALLOCATOR.alloc_with_options(
                    Layout::from_size_align(1234, align)
                        .expect("Failed to create layout"),
                );
                assert!(*e as usize != 0);
                assert!((*e as usize) % align == 0);
            }
        }
    }

    #[test_case]
    fn allocated_object_have_no_overlap() {
//let mut sw = SerialPort::new_for_com1();
        let allocations = [
            Layout::from_size_align(128, 128).unwrap(),
            Layout::from_size_align(64, 64).unwrap(),
            Layout::from_size_align(8, 8).unwrap(),
/*            Layout::from_size_align(16, 16).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(4, 4).unwrap(),
            Layout::from_size_align(2, 2).unwrap(),
            Layout::from_size_align(600000, 64).unwrap(),
            Layout::from_size_align(64, 64).unwrap(),
            Layout::from_size_align(1, 1).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(3, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(600000, 64).unwrap(),
            Layout::from_size_align(6000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
            Layout::from_size_align(60000, 64).unwrap(),
*/            
        ];

//writeln!(sw, "test allocated_object_have_no_overlap start").unwrap(); 
        let mut pointers = vec![null_mut::<u8>(); allocations.len()];
//writeln!(sw, "Allocation preset {:?}", pointers).unwrap(); 
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            *pointer = ALLOCATOR.alloc_with_options(*layout);
            if *pointer == null_mut() {
                panic!("Allocation failed for layout {:?}", layout);
            }
//writeln!(sw, "allocation result = {:?}  -- layout = {:?}", pointer, layout).unwrap();
            for k in 0..layout.size() {
                unsafe { *(pointer.add(k)) = i as u8 }
            }
//writeln!(sw, "allocation result = {:?}  -- layout = {:?}", pointer, layout).unwrap();
//writeln!(sw, "memory write All Ok -- {:?}", *pointer).unwrap();                
        }
//writeln!(sw, "Allocation All Result {:?}", pointers).unwrap(); 
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
//writeln!(sw, "read result = {:?} ", e).unwrap();                
            let (i, (layout, pointer)) = e;
            //for k in 0..layout.size() {
            //    assert!(unsafe { *pointer.add(k) } == i as u8);
            //}
//writeln!(sw, "memory read All Ok");   
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .step_by(2)
        {
            let (_, (layout, pointer)) = e;
            unsafe { ALLOCATOR.dealloc(*pointer, *layout) }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .skip(1)
            .step_by(2)
        {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!(unsafe { *pointer.add(k) } == i as u8);
            }
        }
        for e in allocations
            .iter()
            .zip(pointers.iter_mut())
            .enumerate()
            .step_by(2)
        {
            let (i, (layout, pointer)) = e;
            *pointer = ALLOCATOR.alloc_with_options(*layout);
            for k in 0..layout.size() {
                unsafe { *pointer.add(k) = i as u8 }
            }
        }
        for e in allocations.iter().zip(pointers.iter_mut()).enumerate() {
            let (i, (layout, pointer)) = e;
            for k in 0..layout.size() {
                assert!( unsafe { *pointer.add(k) } == i as u8);
            }
        }
    }
}