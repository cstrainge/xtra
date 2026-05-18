
// This crate contains code and data structures that are intended to be shared between the
// bootloader and the Kernel.

#![no_std]



/// Handler code for walking the device tree from a memory pointer supplied by the BIOS or
/// system bootloader.
pub mod device_tree;



/// Description of the xtra-shared mount table. It allows the bootloader to communicate the system
/// mount table to the Kernel.
pub mod mount_table;
