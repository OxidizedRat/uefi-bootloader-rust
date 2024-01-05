#![no_std]
#![no_main]
#[allow(unused)]
use core::fmt::Write;
use core::panic::PanicInfo;
use uefi::proto::media::file::File;
use uefi::proto::media::file::FileAttribute;
use uefi::proto::media::file::FileMode;
use uefi::table::boot::MemoryMap;
use uefi::table::boot::OpenProtocolAttributes;
use uefi::table::boot::OpenProtocolParams;
use uefi::table::Runtime;
use uefi::{prelude::*, table::boot::MemoryType};
use xmas_elf::ElfFile;

#[entry]
fn main(handle: Handle, system_table: SystemTable<Boot>) -> Status {
    //get systemtable clone for use with console protocol
    let mut system_table_console = unsafe { system_table.unsafe_clone() };
    //get stdout for write macro
    let stdout = system_table_console.stdout();
    //get systemtable ptr reference for use with fs protocol
    let system_table_fs = unsafe { system_table.unsafe_clone() };
    //get simple file system protocol
    let mut simple_fs =
        match BootServices::get_image_file_system(system_table_fs.boot_services(), handle) {
            Ok(sfs) => sfs,
            Err(why) => {
                writeln! {stdout,"Could not get simple file system protocol{:?}",why}.unwrap();
                loop {}
            }
        };
    //get root directory
    let mut root_dir = match simple_fs.open_volume() {
        Ok(rd) => rd,
        Err(why) => {
            writeln! {stdout," Could not open root directory{:?}",why}.unwrap();
            loop {}
        }
    };

    //open file named kernel
    let file_name = cstr16!("kernel");
    let mut kernel_file = match root_dir.open(file_name, FileMode::Read, FileAttribute::empty()) {
        Ok(kf) => kf,
        Err(why) => {
            writeln! {stdout,"Could not find kernel{:?}",why}.unwrap();
            loop {}
        }
    };
    //allocate pool for file info buffer
    let file_info_buffer_addr = match system_table
        .boot_services()
        .allocate_pool(MemoryType::LOADER_DATA, 128)
    {
        Ok(fib) => fib,
        Err(why) => {
            writeln! {stdout,"Could not allocate pool for file info buffer{:?}",why}.unwrap();
            loop {}
        }
    };
    //convert file_info_buffer into slice
    let file_info_buffer = unsafe { core::slice::from_raw_parts_mut(file_info_buffer_addr, 128) };
    //get file info
    let file_info =
        match kernel_file.get_info::<uefi::proto::media::file::FileInfo>(file_info_buffer) {
            Ok(fi) => fi,
            Err(why) => {
                writeln! {stdout,"Could not get file info{:?}",why}.unwrap();
                loop {}
            }
        };
    let kernel_size = file_info.file_size();
    //free file info buffer
    unsafe {
        match system_table
            .boot_services()
            .free_pool(file_info_buffer_addr)
        {
            Ok(_) => {}
            Err(why) => {
                writeln! {stdout,"Could not free pool for file info buffer{:?}",why}.unwrap();
                loop {}
            }
        };
    }
    writeln! {stdout,"Kernel size: {}",kernel_size}.unwrap();
    //allocate pool for kernel
    let kernel_buffer_addr = match system_table
        .boot_services()
        .allocate_pool(MemoryType::LOADER_DATA, kernel_size as usize)
    {
        Ok(kb) => kb,
        Err(why) => {
            writeln! {stdout,"Could not allocate pool for kernel buffer{:?}",why}.unwrap();
            loop {}
        }
    };

    //convert kernel_buffer into slice
    let kernel_buffer =
        unsafe { core::slice::from_raw_parts_mut(kernel_buffer_addr, kernel_size as usize) };
    //convert kernel_file into regular file
    let mut kernel_file = unsafe { uefi::proto::media::file::RegularFile::new(kernel_file) };
    //read kernel into kernel_buffer
    match kernel_file.read(kernel_buffer) {
        Ok(_) => {}
        Err(why) => {
            writeln! {stdout,"Could not read kernel into kernel buffer{:?}",why}.unwrap();
            loop {}
        }
    };
    //parse kernel as elf
    let kernel_elf = match ElfFile::new(kernel_buffer) {
        Ok(ke) => ke,
        Err(why) => {
            writeln! {stdout,"Could not parse kernel as elf{:?}",why}.unwrap();
            loop {}
        }
    };

    //iterate over program headers and get headers of type load
    for ph in kernel_elf.program_iter() {
        if ph.get_type() == Ok(xmas_elf::program::Type::Load) {
            //get segment size
            let segment_size = ph.mem_size() as usize;
            //allocate pool for segment
            let segment_buffer_addr = match system_table
                .boot_services()
                .allocate_pool(MemoryType::LOADER_DATA, segment_size)
            {
                Ok(sb) => sb,
                Err(why) => {
                    writeln! {stdout,"Could not allocate pool for segment buffer{:?}",why}.unwrap();
                    loop {}
                }
            };
            //convert segment_buffer into slice
            let segment_offset = ph.offset() as usize;
            let segment_buffer =
                unsafe { core::slice::from_raw_parts_mut(segment_buffer_addr, segment_size) };
            //copy segment into segment_buffer
            segment_buffer
                .copy_from_slice(&kernel_buffer[segment_offset..segment_offset + segment_size]);
            //get segment destination address
            let segment_dest_addr = ph.physical_addr() as usize;
            //calculate number of pages required for segment
            let num_pages = (segment_size + 0xfff) / 0x1000;
            //get aligned segment destination address
            let aligned_segment_dest_addr = segment_dest_addr - (segment_dest_addr % 4096);
            //allocate pages for segment
            match system_table.boot_services().allocate_pages(
                uefi::table::boot::AllocateType::Address(aligned_segment_dest_addr as u64),
                MemoryType::LOADER_CODE,
                num_pages,
            ) {
                Ok(addr) => addr,
                Err(why) => {
                    writeln! {stdout,"Could not allocate pages for segment{:?}",why}.unwrap();
                    loop {}
                }
            };
            //zero out allocated pages
            unsafe {
                core::ptr::write_bytes(aligned_segment_dest_addr as *mut u8, 0, num_pages * 0x1000);
            }
            //copy segment into segment_dest_addr
            unsafe {
                core::ptr::copy(
                    segment_buffer_addr as *const u8,
                    segment_dest_addr as *mut u8,
                    segment_size,
                );
            }
            //free segment buffer
            unsafe {
                match system_table.boot_services().free_pool(segment_buffer_addr) {
                    Ok(_) => {}
                    Err(why) => {
                        writeln! {stdout,"Could not free pool for segment buffer{:?}",why}.unwrap();
                        loop {}
                    }
                };
            }
        }
    }
    writeln! {stdout,"Kernel loaded"}.unwrap();

    //get handle for gop
    let gop_handle = match system_table
        .boot_services()
        .get_handle_for_protocol::<uefi::proto::console::gop::GraphicsOutput>()
    {
        Ok(gh) => gh,
        Err(why) => {
            writeln! {stdout,"Could not get handle for gop{:?}",why}.unwrap();
            loop {}
        }
    };
    //get gop
    let params: OpenProtocolParams = OpenProtocolParams {
        handle: gop_handle,
        agent: handle,
        controller: None,
    };
    let system_table_gop = unsafe { system_table.unsafe_clone() };
    let mut gop = unsafe {
        match system_table_gop
            .boot_services()
            .open_protocol::<uefi::proto::console::gop::GraphicsOutput>(
                params,
                OpenProtocolAttributes::GetProtocol,
            ) {
            Ok(g) => g,
            Err(why) => {
                writeln! {stdout,"Could not get gop{:?}",why}.unwrap();
                loop {}
            }
        }
    };
    writeln! {stdout,"Got gop"}.unwrap();
    //get framebuffer address
    let framebuffer_addr = gop.frame_buffer().as_mut_ptr();
    let framebuffer_size = gop.frame_buffer().size();
    let (framebuffer_width, framebuffer_height) = gop.current_mode_info().resolution();
    let stride = gop.current_mode_info().stride();
    //create framebuffer info struct
    let framebuffer_info = FramebufferInfo {
        framebuffer_addr,
        framebuffer_size,
        framebuffer_width,
        framebuffer_height,
        stride,
    };
    //get kernel entry point
    let kernel_entry_point = kernel_elf.header.pt2.entry_point() as usize;
    writeln! {stdout,"Kernel entry point: {:#x}",kernel_entry_point}.unwrap();
    //define kernel entry point type
    type KernelMain = extern "efiapi" fn(
        fb_info: FramebufferInfo,
        system_table: SystemTable<Runtime>,
        MemoryMap,
    ) -> !;
    //comvert kernel entry point into KernelMain
    let kernel_entry_point = kernel_entry_point as *const ();
    let kernel_main: KernelMain = unsafe { core::mem::transmute(kernel_entry_point) };

    writeln! {stdout,"Calling kernel entry point"}.unwrap();
    //exit boot services
    let (system_table_runtime, memory_map) =
        system_table.exit_boot_services(MemoryType::LOADER_DATA);

    kernel_main(framebuffer_info, system_table_runtime, memory_map);
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[repr(C)]
#[derive(Debug)]
pub struct FramebufferInfo {
    pub framebuffer_addr: *mut u8,
    pub framebuffer_size: usize,
    pub framebuffer_width: usize,
    pub framebuffer_height: usize,
    pub stride: usize,
}
