#![no_std]
#![no_main]
#![feature(abi_efiapi)]
use core::{cell::UnsafeCell, *};
extern crate alloc;
use alloc::vec::Vec;
use alloc::{boxed::Box, vec};
use log::{error, info};
use uefi::{prelude::*, proto::console::gop::*, proto::media::file::*, table::boot::*};
use xmas_elf::{program::*, *};

#[entry]
fn main(handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi_services::init(&mut system_table).unwrap();

    //get systemtable ptr reference for use with fs protocol
    let system_table_fs = uefi_services::system_table().as_ptr();
    //get simple file system protocol
    let simple_fs = unsafe {
        match BootServices::get_image_file_system((*system_table_fs).boot_services(), handle) {
            Ok(sfs) => sfs,
            Err(why) => {
                error! {"{:?}",why};
                loop {}
            }
        }
    };
    //open the root directory
    let mut directory: Directory = unsafe { (*simple_fs.interface.get()).open_volume().unwrap() };
    //open file named kernel
    let kernel_path = cstr16!("kernel");
    let kernel_handle = match directory.open(kernel_path, FileMode::Read, FileAttribute::empty()) {
        Ok(handle) => handle,
        Err(why) => {
            error!("Could not find kernel {:?}", why);
            loop {}
        }
    };
    let mut kernel_file = match kernel_handle.into_regular_file() {
        Some(reg_file) => reg_file,
        None => {
            error!("Kernel is not a file");
            loop {}
        }
    };
    info!("Kernel Found");
    //get kernel info, status 4 is buffer_too_small
    //first run with a blank buffer to get the required size
    let mut kernel_info_buffer: Vec<u8> = Vec::new();
    let mut req_size = 0;
    match kernel_file.get_info::<FileInfo>(&mut kernel_info_buffer) {
        Ok(_) => (),
        Err(err) if err.status() == Status::BUFFER_TOO_SMALL => req_size = err.data().unwrap(),
        Err(why) => {
            error!("{:?}", why);
            loop {}
        }
    };
    //set required size and try again
    kernel_info_buffer.resize(req_size, 0);
    let kernel_info = match kernel_file.get_info::<FileInfo>(&mut kernel_info_buffer) {
        Ok(info) => info,
        Err(why) => {
            error!("2{:#?}", why);
            loop {}
        }
    };
    //get size of kernel and create buffer to read kernel into
    let kernel_size = kernel_info.file_size();
    let mut kernel_buffer: Vec<u8> = vec![0; kernel_size.try_into().unwrap()];
    match kernel_file.read(&mut kernel_buffer) {
        Ok(bytes_read) => info!("Read {} bytes from kernel", bytes_read),
        Err(why) => {
            error!("Failed to read kernel!:{:?}", why);
            loop{}
        }
    }
    //parse buffer with xmas elf
    let kernel_elf = match ElfFile::new(&kernel_buffer) {
        Ok(elf) => elf,
        Err(why) => {
            error!("{}", why);
            loop{}
        }
    };
    //get kernel entry point ptr
    let entry_point = kernel_elf.header.pt2.entry_point();
    //create a buffer to hold vecs of loaded section
    //unsure if the destructor for the loaded sections will be called therefore adding them
    //to this vec
    let mut loaded_sections: Vec<Vec<u8>> = Vec::new();
    //iterate over the program headers and load each header of type load
    for header in kernel_elf.program_iter() {
        if header.get_type().unwrap() == Type::Load {
            let virt_addr = header.virtual_addr();
            let file_size: usize = header.file_size().try_into().unwrap();
            let mem_size = header.mem_size();
            let file_offset: usize = header.offset().try_into().unwrap();
            //align address on 4k boundary
            let address = virt_addr - (virt_addr % 4096);
            let mem_size_actual = (virt_addr - address) + mem_size;
            //compute number of pages required
            let num_pages: usize = ((mem_size_actual / 4096) + 1).try_into().unwrap();
            //get ptr to system table for allocate pages call
            let table = uefi_services::system_table().as_ptr();
            unsafe {
                let ptr = match BootServices::allocate_pages(
                    (*table).boot_services(),
                    AllocateType::Address(address.try_into().unwrap()),
                    MemoryType(2),
                    num_pages,
                ) {
                    Ok(addr) => addr,
                    Err(why) => {
                        error!("Failed to allocate pages:{:?}", why);
                        loop {}
                    }
                };
                //create a vec from allocated buffer and fill it with zeros
                let mut buffer =
                    Vec::from_raw_parts(ptr as *mut u8, num_pages * 4096, num_pages * 4096);
                for byte in buffer.iter_mut() {
                    *byte = 0;
                }
                //compute offset of the data with in the kernel buffer to be copied to memory
                let offset: usize = (virt_addr - address).try_into().unwrap();
                //start index for buffer
                let mut start_index: usize = (offset).try_into().unwrap();
                for byte in &kernel_buffer[file_offset..(file_offset + file_size)] {
                    buffer[start_index] = *byte;
                    start_index += 1;
                }
                //unsure if destructor will be called for our buffer when it goes out of scope,
                // therefore push it to a vector outside of for loop
                //global allocator's dealloc may be capable of deallocating our buffer
                loaded_sections.push(buffer);
            }
        }
    }
    //our kernel's entry point's prototype
    type KernelMain = fn(frame_buffer: &mut FrameBuffer, mem_map_buf: &mut [u8]) -> !;
    let kernel_main: KernelMain;
    unsafe {
        kernel_main = core::mem::transmute(entry_point);
    }
    let gop = get_gop(&system_table).get();

    let framebuffer = FrameBuffer::new(gop);
    let framebuffer = Box::leak(framebuffer);
    info!("Calling kernel");
    //get memory map size and create a buffer for memory map before calling exit boot services
    let size = BootServices::memory_map_size(&system_table.boot_services());
    let map_size = size.map_size;
    let entry_size = size.entry_size;
    let mut mem_map_buf: Vec<u8> = Vec::new();
    mem_map_buf.resize(map_size + entry_size * 10, 0);
    //exit boot services
    system_table.exit_boot_services(handle, &mut mem_map_buf);
    kernel_main(framebuffer, &mut mem_map_buf);
    //kernel never returns so we should never get here
  
}
//get graphics output protocol
fn get_gop(sys: &SystemTable<Boot>) -> &UnsafeCell<GraphicsOutput> {
    let gop = BootServices::locate_protocol::<GraphicsOutput>(sys.boot_services()).unwrap();
    return gop;
}

struct FrameBuffer {
    _base_address: *mut u8,
    _size: usize,
    _width: usize,
    _height: usize,
    _stride: usize,
}

impl FrameBuffer {
    pub fn new(gop: *mut GraphicsOutput) -> Box<FrameBuffer> {
        let base_address = unsafe { (*gop).frame_buffer().as_mut_ptr() };
        let size = unsafe { (*gop).frame_buffer().size() };
        let (width, height) = unsafe { (*gop).current_mode_info().resolution() };
        let stride = unsafe { (*gop).current_mode_info().stride() };
        let fb = FrameBuffer {
            _base_address: base_address,
            _size: size,
            _width: width,
            _height: height,
            _stride: stride,
        };

        let fb_heap = Box::new(fb);
        fb_heap
    }
}
