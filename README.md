# uefi-bootloader-rust
a uefi bootloader written in rust for a personal project


## What it does ##
Loads a elf binary named "kernel" from the root directory of the efi partition and transfers control to it.


## Building ##
The build.sh file contains the following cargo command
```
cargo +nightly build  -Z build-std=core,compiler_builtins,alloc,panic_abort,std -Z build-std-features=compiler-builtins-mem  --target x86_64-unknown-uefi
```

## To do ##

*   Initialize GOP
*   pass references to the system table and GOP to the kernel