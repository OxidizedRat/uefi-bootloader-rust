#![no_std]
#![no_main]
#![feature(abi_efiapi)]
#![feature(default_alloc_error_handler)]

use core::panic::PanicInfo;
mod uefi;
use crate::uefi::*;
use core::ffi::c_void;
use core::fmt::Write;
use core::mem::transmute;
use xmas_elf::ElfFile;

#[no_mangle]
pub extern "efiapi" fn efi_main(_handle:Handle,system_table:*const SystemTable)->Status{
    //init Writer
    let mut writer = Writer::init(system_table);
    let rust_string = "Hello world!\n\r";
    write!(&mut writer, "{}",rust_string).expect("error");
    let kernel = match get_kernel_file(system_table){
        Ok(proto) => proto,
        Err(why) => {
                write!(&mut writer,"Could not get kernel, error :{}\n\r",why).expect("write error");
                loop{};
        }
    };
    let kernel_size =  match get_kernel_size(system_table,kernel){
        Ok(size)    => size,
        Err(why)    => {
            write!(&mut writer,"Could not get kernel size, Error :{}\n\r",why).expect("write error");
            loop{};
        }
    };
    write!(writer,"kernel size: {}\n\r",kernel_size).expect("write error");
    //allocate memory for buffer
    let kernel_buffer_ptr = match allocate_pool(system_table,kernel_size){
        Ok(ptr)     => ptr,
        Err(why)    => {
            write!(&mut writer,"Could not Allocate memory for kernel, Error :{}\n\r",why).expect("write error");
            loop{};
        }
    };

    let fill_kernel_buffer =get_kernel_buffer(kernel,&kernel_size,kernel_buffer_ptr);
    if fill_kernel_buffer !=0{
        write!(&mut writer,"could not read kernel file, Error:{}",fill_kernel_buffer).expect("write error");
        loop{};
    }
    let kernel_buffer  = unsafe{core::slice::from_raw_parts(kernel_buffer_ptr,kernel_size)};


    //parse our buffer
    let kernel_elf = match ElfFile::new(kernel_buffer){
        Ok(elf)   => elf,
        Err(why)  => {
            write!(&mut writer,"Failed to read kernel binary: {}\n\r",why).expect("");
            return 1;
        }
    };
    
    //load the load type headers into memory
    for p_header in kernel_elf.program_iter(){
        let p_type = match p_header.get_type(){
            Ok(header) => header,
            Err(_)     => continue,
        };
        //iterate trough the program headers and get the headers
        //with type load
        if p_type == xmas_elf::program::Type::Load{

            let mut address = p_header.physical_addr();
            let mut offset_mem:u64 = 0;
            //make the address 4096 aligned
            if address % 4096 !=0{
                offset_mem = address%4096;
                address = address - offset_mem;
            }
            let status = allocate_pages(system_table,&address,1);

            if status !=0{
                write!(&mut writer,"Address:{:#0x}, Error code:{:#0x}\n\r",address,status).expect("write error");
                loop{}
            }
            //get a slice to the page we just allocated
            let load_slice = unsafe{
                core::slice::from_raw_parts_mut(
                    (address+offset_mem) as *mut u8, 
                    4096-offset_mem as usize)
            };
            //zero out the slice
            for byte in load_slice.iter_mut(){
                *byte = 0;
            }
            //fill our slice with the required data
            let offset = p_header.offset() as usize;

            let header_size = p_header.file_size() as usize + offset;

            let mut kernel_index = 0;
            let mut load_index = 0;
            for byte in kernel_buffer{
                if kernel_index >=offset && kernel_index<=header_size{
                    load_slice[load_index] = *byte;
                    load_index+=1;
                }
                kernel_index+=1;
                
            }
        }

    }
    //get the entry point
    let entry_point:u64 = kernel_elf.header.pt2.entry_point();
    //convert address to function pointer
    let exec_kernel: fn(system_table:*const SystemTable,memory_map:(*const u8,usize,usize)) -> i32 = unsafe{transmute(entry_point)};

    //get memory map
    let mem_map = match get_memory_map(system_table, &mut writer){
        Ok(mem_tuple)   => mem_tuple,
        Err(why)        => {
            write!(&mut writer,"failed to get memory map,Error: {:#0x}",why).expect("");
            loop{}
        }
    };


    //run kernel
    let return_val = exec_kernel(system_table,mem_map);
    write!(&mut writer,"kernel returned: {}",return_val).expect("write error");
    
    
    loop{}
    return 1;
}
fn get_memory_map(system_table:*const SystemTable,writer:&mut Writer) -> Result<(*const u8,usize,usize),Status>{
    //fn(memory_map_size:*const  usize,memory_map:*const u8,map_key:*const usize,descriptor_size:*const usize,descriptor_version:*const usize) ->Status,
    let mut memory_map_size:usize = 0;
    //null pointer should not get accessed on the first call as memory map size is set to 0
    let memory_map:*const u8 = unsafe{transmute(memory_map_size)};
    let map_key:usize = 0;
    let descriptor_size:usize = 0;
    let descriptor_version:usize = 0;

    //call get memory map once with size set to 0 so that it returns the required buffer size
    unsafe{
        let _status = ((*(*system_table).boot).get_memory_map)(
                &memory_map_size,
                memory_map,
                &map_key,
                &descriptor_size,
                &descriptor_version
        );
    }
    //increment to ensure we have enough space
    memory_map_size+=1;
    //allocate buffer for memory map
    
    let buffer = match allocate_pool::<u8>(system_table,memory_map_size) {
            Ok(buff)    => buff,
            Err(why)    => {
                write!(writer,"Failed to allocate pool, Error :{:#0x}",why).expect("write error");
                return Err(why);
            }
    };
    //call get memory map again with proper size
    let status = unsafe{
            ((*(*system_table).boot).get_memory_map)(
                    &memory_map_size,
                    buffer,
                    &map_key,
                    &descriptor_size,
                    &descriptor_version
            )
    };
    if status!=0{
        return Err(status);
    }

    Ok((buffer,memory_map_size,map_key))
}
fn allocate_pool<T>(system_table:*const SystemTable,size:usize) ->Result<*const T,Status>{
    //extern "efiapi" fn(pool_type:MemoryType,size: usize,buffer:*const *const c_void)-> Status,

    let buffer:&&u64 = &&0;
    let buffer_ptr: *const *const c_void = unsafe{transmute(buffer)};
    let status = unsafe{((*(*system_table).boot).allocate_pool)(MemoryType::EfiLoaderData,
                                                                size,
                                                                buffer_ptr)};
    if status !=0{
        return Err(status);
    }
    let output_buffer:*const T = unsafe{transmute(*buffer_ptr)};
    return Ok(output_buffer);
}
fn get_kernel_file(system_table:*const SystemTable) -> Result<*const *const FileProtocol,Status>{
    let guid = GUID{
        data1:0x0964e5b22,
        data2:0x6459,
        data3:0x11d2,
        data4:[0x8e,0x39,0x00,0xa0,0xc9,0x69,0x72,0x3b],
    };
    //locate Simple file system protocol
    let interface:&&u128=&&0;
    let interface_void:*const *const c_void = unsafe{core::mem::transmute(interface)};
    let fs_status = unsafe{((*(*system_table).boot).locate_protocol)(&guid,0,interface_void)};
    if fs_status != 0{
        return Err(1);
    }
    //open root volume handle using simple file system
    let simple_file_system:*const *const SimpleFileSystemProtocol = unsafe{core::mem::transmute(interface_void)};
    let file_p:&&[u8;120] = &&[0;120];
    let file_protocol:*const *const FileProtocol = unsafe{core::mem::transmute(file_p)};
    let file_status = unsafe{((*(*simple_file_system)).open_volume)(*simple_file_system,file_protocol)};

    if file_status !=0{
        return Err(2);
    }
    //get the kernel file using root volume handle
    let kernel_name:&[u16;6] = &['k' as u16,'e' as u16,'r' as u16,'n' as u16,'e' as u16,'l' as u16];
    let kernel_file_handle_buffer:&&[u8;120] = &&[0;120];
    let kernel_file_handle:*const *const FileProtocol =unsafe{transmute(kernel_file_handle_buffer)};
    //open kernel file in read mode
    let read_status = 
    unsafe{((*(*file_protocol)).open)(*file_protocol,kernel_file_handle,kernel_name.as_ptr(),0x0000000000000001,0)};

    if read_status!=0{
        return Err(4);
    }
    return Ok(kernel_file_handle);
}
fn get_kernel_size(system_table:*const SystemTable,kernel_file_handle:*const *const FileProtocol) ->Result<usize,Status>{

    //file info GUID
    let file_info_guid = GUID{
        data1:0x09576e92,
        data2:0x6d3f,
        data3:0x11d2,
        data4:[0x8e,0x39,0x00,0xa0,0xc9,0x69,0x72,0x3b],
    };
    let allocated_address:&&u64 = &&0;
    let allocated_address:*const *const c_void = unsafe{transmute(allocated_address)};
    //allocate memory for fileinfo struct
    let status_pool = unsafe{((*(*system_table).boot).allocate_pool)(MemoryType::EfiLoaderData,200,allocated_address)};

    if status_pool!=0{
        return Err(3);
    }

    //get file info struct
    let file_info_status = unsafe{((*(*kernel_file_handle)).get_info)(*kernel_file_handle,&file_info_guid,&200,*allocated_address)};
    if file_info_status!=0{
        return Err(5);
    }
    let file_info_struct:*const FileInfo = unsafe{transmute(*allocated_address)};

    return Ok(unsafe{(*file_info_struct).file_size.try_into().unwrap()});
}

fn get_kernel_buffer(kernel_file_handle:*const *const FileProtocol,kernel_size:*const usize,buffer_ptr:*const u8)->Status{
    //prepare buffer
    let kernel_buffer_void: *const c_void = unsafe{transmute(buffer_ptr)};
    //read the contents into a buffer
    let status = unsafe{((*(*kernel_file_handle)).read)(*kernel_file_handle,kernel_size,kernel_buffer_void)};

    return status;
}

extern "efiapi" fn allocate_pages(system_table:*const SystemTable,address:*const u64,size:usize)->Status{

    let status = unsafe{
        ((*(*system_table).boot).allocate_pages)(AllocType::AllocateAddress,
                                                 MemoryType::EfiLoaderData,
                                                 size,
                                                 address)
    };
    status
}

/*
fn get_gop(system_table:&SystemTable){
    let guid = GUID{
        data1:0x9042a9de,
        data2:0x23dc,
        data3:0x4a38,
        data4:[0x96,0xfb,0x7a,0xde,0xd0,0x80,0x51,0x6a],
    };

    let interface:&&u128=&&0;
    let interface_void:*const *const c_void = unsafe{core::mem::transmute(interface)};
    let _gop = unsafe{((*system_table.boot).locate_protocol)(&guid,0,interface_void)};
}
*/

fn _free_pool<T>(system_table:*const SystemTable,buffer:*const T) ->Result<(),Status> {
        let buffer_void:*const c_void = unsafe{transmute(buffer)};
        let free_mem_status = unsafe{
            ((*(*system_table).boot).free_pool)(buffer_void)
        };

        if free_mem_status!=0{
            return Err(free_mem_status);
        }

        Ok(())
}
#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
	loop{}
}

struct Writer{
    system_table:*const SystemTable,
}
impl Writer{
    fn init(system_table:*const SystemTable) -> Writer{
        Writer{
            system_table: system_table,
        }
    }
}
impl Write for Writer{
    fn write_str(&mut self,s:&str) -> core::fmt::Result{

        //using allocate pool for this buffer causes the output text to 
        // to have invalid unicode characters not sure why
        let buffer:&mut [u16;512] = &mut [0;512];
        let mut counter = 0;
        for chars in s.chars(){
            buffer[counter] = chars as u16;
            counter +=1;
        }

        unsafe{
            ((*(*self.system_table).output).output_string)((*self.system_table).output,&buffer[0]);
        }

        Ok(())
    }
}
