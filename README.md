# uefi-bootloader-rust
a uefi bootloader written in rust for a personal project


## What it does ##
Loads an elf binary named "kernel" from the root directory of the efi partition and transfers control to it.

Passes reference to system table and memory map to entry point of kernel



## Building ##
The build.sh file contains the following cargo command
```
cargo +nightly build  -Z build-std=core,compiler_builtins,alloc,panic_abort,std -Z build-std-features=compiler-builtins-mem  --target x86_64-unknown-uefi
```

## To do ##

*   Initialize GOP
*   Pass GOP reference to kernel
