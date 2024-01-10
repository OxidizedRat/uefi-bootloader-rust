pub struct Elf64Rela {
    pub r_offset: u64,
    pub r_info: u64,
    pub r_addend: i64,
}
impl Elf64Rela {
    //expects file to be already loaded into memory
    pub fn relocate(&self, base_address: usize) -> Result<(), &'static str> {
        match self.r_info & 0xffffffff {
            8 => (), // R_X86_64_RELATIVE
            _ => return Err("Unsupported relocation type"),
        };
        //get the address of the relocated symbol
        let addend_usize = usize::try_from(self.r_addend).unwrap();

        let result_addr = base_address + addend_usize;
        //edit the file buffer with the relocated address
        let offset_usize = self.r_offset as usize;
        let result_addr_bytes = result_addr.to_le_bytes();
        let relocatable_field_address = unsafe {
            core::slice::from_raw_parts_mut(
                (base_address + offset_usize) as *mut u8,
                core::mem::size_of::<usize>(),
            )
        };
        relocatable_field_address.copy_from_slice(&result_addr_bytes);
        Ok(())
    }
}
