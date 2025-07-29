
/// Module for the RISC-V 64-bit Memory Management Unit (MMU).



/// The RISC-V 64-bit architecture uses a page size of 4KB, so we define it here.
pub const PAGE_SIZE: usize = 4096;



// Make sure that the kernel's configured page size matches the RISC-V 64-bit page size. If building
// for a different architecture, this assertion will never be reached.
const _: () =
    {
        assert!(crate::memory::PAGE_SIZE == PAGE_SIZE,
                "The page size in the memory module must match the RISC-V 64-bit page size.");
    };



/// This module provides the implementation of the MMU for the RISC-V 64-bit architecture using the
/// SV39 page table format. It defines the page table entry structure and the constants used for
/// managing the page table entries.
#[cfg(feature = "sv39")]
mod sv39
{
    /// The definition of the page table entry structure for the SV39 page table format.
    pub mod page_table_entry;


    /// The definition of the virtual address structure for the SV39 page table format.
    pub mod virtual_address;

    /// The definition of the page table structure for the SV39 page table format.
    pub mod page_table;
}


#[cfg(feature = "sv39")]
pub use sv39::*;



// TODO: Add the other formats for the page tables we want to support in the future.

// Ex:
// use sv32::*;
// use sv48::*;


/*
#[cfg(target_arch = "riscv64")]
pub use crate::arch::mmu::sv39::{Sv39PageTable as PageTable, ...};

#[cfg(target_arch = "aarch64")]
pub use crate::arch::mmu::armv8::{ArmV8PageTable as PageTable, ...};
*/
