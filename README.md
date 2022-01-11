# uefi-bootloader-rust
a UEFI bootloader written in rust for a personal project


## What it does ##
Loads an elf binary named "kernel" from the root directory of the efi partition and transfers control to it.
### How it does it ###
*   Loads kernel into memory.
*   Gets GOP and sets mode to the highest resolution.
*   Gets address of framebuffer.
*   Gets memory map.
*   Exits boot services.
*   Calls kernel entry point and passes pointer to system table along with GOP and memory map.

## Building ##
The build.sh file contains the following cargo command:
```
cargo +nightly build  -Z build-std=core,compiler_builtins,alloc,panic_abort,std -Z build-std-features=compiler-builtins-mem  --target x86_64-unknown-uefi
```

## To do ##

* kernel returns garbage, likely issue with how it is loaded into memory.
