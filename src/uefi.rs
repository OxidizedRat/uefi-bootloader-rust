
use core::ffi::c_void;

pub type Handle = u64;
pub type Status = usize;
type PhysicalAddress = u64;
#[repr(C)]
pub struct TableHeader{
    signature: u64,
    revision: u32,
    header_size: u32,
    crc32:      u32,
    reserved: u32,
}

#[repr(C)]
pub struct SystemTable{
    header: TableHeader,
    firmware_vendor: *const u16, //supposed to be long char
    firmware_revision:u32,
    input_handle: Handle,
    input: *const TextInputProtocol,
    output_handle: Handle,
    pub output: * const TextOutputProtocol,
    error_handle: Handle,
    error:  *const ErrorOutputProtocol,
    runtime: *const RuntimeServices,
    pub boot: *const BootServices,
    no_of_entries: usize,
    config_table: *const ConfigurationTable,
}

#[repr(C)]
pub struct TextInputProtocol{
}
//u64 are pointers to functions that will not be used
#[repr(C)]
pub struct TextOutputProtocol{
    reset: extern "efiapi" fn(output_protocol:&TextOutputProtocol,verification:u8)-> Status,
    pub output_string: extern "efiapi" fn(output_protocol:*const TextOutputProtocol,string :*const u16)-> Status,
    test_output: u64,
    query_mode: u64,
    set_mode: u64,
    set_attribute: u64,
    pub clear_screen:  extern "efiapi" fn(output_protocol:*const TextOutputProtocol)->Status,
    set_cursor_position: u64,
    enable_cursor: u64,
    mode: *const TextMode,
}

#[repr(C)]
pub struct TextMode{
    _non_exhaustive: ()
}

#[repr(C)]
pub struct ErrorOutputProtocol{
    _non_exhaustive: ()
}
#[repr(C)]
pub struct RuntimeServices{
    _non_exhaustive: ()
}
//u64s are function pointers
#[repr(C)]
pub struct BootServices{
    header:                         TableHeader,
    //tpl services
    raise_tpl:                      u64,
    restore_tpl:                    u64,
    //memory related services
    pub allocate_pages:             extern "efiapi" fn(allocate_type:AllocType,mem_type:MemoryType,pages:usize,address:* const PhysicalAddress)->Status,
    free_pages:                     extern "efiapi" fn(memory:u64,pages:usize)->Status,
    pub get_memory_map:             extern "efiapi" fn(memory_map_size:*const  usize,memory_map:*const u8,map_key:*const usize,descriptor_size:*const usize,descriptor_version:*const usize) ->Status,
    pub allocate_pool:              extern "efiapi" fn(pool_type:MemoryType,size: usize,buffer:*const *const c_void)-> Status,
    pub free_pool:                  extern "efiapi" fn(buffer:*const c_void)->Status,
    //event services
    create_event:                   u64,
    set_timer:                      u64,
    wait_for_event:                 u64,
    signal_event:                   u64,
    close_event:                    u64,
    check_event:                    u64,
    //protocol handlers
    install_protocol_interface:     u64,
    reinstall_protocol_interface:   u64,
    uninstall_protocol_interface:   u64,
    handle_protocol:                u64,
    reserved:                       *const c_void,
    register_protocol_notify:       u64,
    locate_handle:                  extern "efiapi" fn(search_type:SearchType,guid:*const GUID,key:*const c_void,buffer_size:*const usize,handle :*const Handle)->Status,
    locate_device_path:             u64,
    install_config_table:           u64,
    //image services
    image_load:                     u64,
    image_start:                    u64,
    exit:                           u64,
    image_unload:                   u64,
    exit_boot_services:             u64,
    //miscellaneous
    get_next_monotonic_count:       u64,
    stall:                          u64,
    set_watchdog_timer:             u64,
    //driver support
    connect_controller:             u64,
    disconnect_controller:          u64,
    //open and close protocol service
    open_protocol:                  u64,
    close_protocol:                 u64,
    open_protocol_information:      u64,
    //Library services
    protocols_per_handle:           u64,
    locate_handle_buffer:           u64,
    pub locate_protocol:                extern "efiapi" fn(guid:*const GUID,registration:u64,interface:*const *const c_void) ->Status,
    install_multiple_protocol_inter:u64,
    uninstall_above:                u64,
    //crc services
    calculate_crc:                  u64,
    //miscellaneous again
    copy_mem:                       u64,
    set_mem:                        u64,
    create_event_ex:                u64,
}
#[repr(C)]
pub struct _MemoryMapDescriptor{

}
#[repr(C)]
pub struct GUID{
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8;8],
}
#[repr(C)]
pub enum AllocType{
    AllocateAnyPages,
    AllocateMaxAddress,
    AllocateAddress,
    MaxAllocateType,
}
#[repr(C)]
pub enum MemoryType{
    EfiReservedMemoryType,
    EfiLoaderCode,
    EfiLoaderData,
    EfiBootServicesCode,
    EfiBootServicesData,
    EfiRuntimeServicesCode,
    EfiRuntimeServicesData,
    EfiConventionalMemory,
    EfiUnusableMemory,
    EfiACPIReclaimMemory,
    EfiACPIMemoryNVS,
    EfiMemoryMappedIO,
    EfiMemoryMappedIOPortSpace,
    EfiPalCode,
    EfiPersistentMemory,
    EfiUnacceptedMemoryType,
    EfiMaxMemoryType,
}


#[repr(C)]
pub enum SearchType{
    _AllHandles,
    _ByRegisterNotify,
    _ByProtocol,
}


#[repr(C)]
pub struct ConfigurationTable{
    _non_exhaustive: ()
}

#[repr(C)]
pub struct SimpleFileSystemProtocol{
    revision:   u64,
    pub open_volume: extern "efiapi" fn(this:*const SimpleFileSystemProtocol,root:*const *const FileProtocol) -> Status,
}

#[repr(C)]
pub struct FileProtocol{
    revision: u64,
    pub open:   extern "efiapi" fn(this:*const FileProtocol, new_handle:*const *const FileProtocol,name:*const u16,open_mode:u64,attribute:u64)-> Status,
    close:  u64,
    delete: u64,
    pub read:   extern "efiapi" fn(this:*const FileProtocol,buffer_size:*const usize,buffer:*const c_void)->Status,
    write:  u64,
    get_position:   u64,
    set_position:   u64,
    pub get_info:       extern "efiapi" fn(this:*const FileProtocol,info_type:&GUID,buffer_size:*const usize,buffer:*const c_void)->Status,
    set_info:       u64,
    flush:  u64,
    open_ex:    u64,
    read_ex:    u64,
    write_ex:   u64,
    flush_ex:   u64,
}

#[repr(C)]
pub struct _GraphicsOutputProtocol{
    query_mode: u64,
    set_mode:   u64,
    blt:        u64,
    mode:   *const _GOPMode,
}
#[repr(C)]
pub struct _GOPMode{
    max_mode:   u32,
    mode:   u32,
    info: *const _GOPModeInformation,
    size_of_info:   usize,
    framebuffer_base:   PhysicalAddress,
    framebuffer_size:   usize,
}
#[repr(C)]
pub struct _GOPModeInformation{
    version:    u32,
    horizontal_resolution:  u32,
    vertical_resolution:    u32,
    pixel_format:   _PixelFormat,
    pixel_information:  _PixelBitmask,
    pixels_per_line:    u32,
}

 #[repr(C)]
 pub enum _PixelFormat{
     PixelRedGreenBlueReserved8BitPerColor,
     PixelBlueGreenRedReserved8BitPerColor,
     PixelBitMask,
     PixelBltOnly,
     PixelFormatMax,
 }
#[repr(C)]
pub struct _PixelBitmask{
    red_mask:   u32,
    green_mask: u32,
    blue_mask:  u32,
    reserved_mask:  u32,
}
#[repr(C)]
#[derive(Debug)]
pub struct FileInfo{
    size: usize,
    pub file_size: u64,
    physical_size:  u64,
    create_time:    EfiTime,
    last_access_time:   EfiTime,
    modification_time:  EfiTime,
    attribute:  u64,
    pub file_name:  *const u16,
}

#[repr(C)]
#[derive(Debug)]
pub struct EfiTime{
    data:[u8;16],
}
