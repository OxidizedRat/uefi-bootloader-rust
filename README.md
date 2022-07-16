# uefi-bootloader-rust
a UEFI bootloader written in rust for a personal project


## What it does ##
Loads an elf binary named "kernel" from the root directory of the efi partition and transfers control to it.
### How it does it ###
*   Loads kernel into memory.
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
  fn(frame_buffer: &mut FrameBuffer, mem_map_buf: &mut [u8]) -> !;
```

FrameBuffer is defined on line 169 in main.rs
