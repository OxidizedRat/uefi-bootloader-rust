# uefi-bootloader-rust
a UEFI bootloader written in rust for a personal project


## What it does ##
Loads an elf binary named "kernel" from the root directory of the efi partition and transfers control to it.
### How it does it ###
*   Loads kernel into memory.
*   Relocates kernel if necessary
*   Gets GOP.
*   Gets framebuffer information.
*   exit boot services is called, which also gives us the memory map.
*   Calls kernel entry point and passes pointer to framebuffer information and memory map.

## Building ##
The following command should suffice:
```
cargo build
```
## Expected Kernel Entry Point ##

```Rust
  extern "efiapi" fn(fb_info:FramebufferInfo,system_table:SystemTable<Runtime>,memory_map:&MemoryMap) -> ! 
```

