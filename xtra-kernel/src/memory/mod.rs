
// The memory module provides the kernel with the ability to manage memory, including allocating
// and freeing memory as well as managing the virtual memory space of the kernel and user processes.



// Important memory constants.
const PAGE_SIZE: usize = 4096;  // Our memory is split into 4KB pages.



// The sub-modules of the memory subsystem.
pub mod kernel;      // Information about the kernel's memory usage and layout.
pub mod memory_device;  // The RAM device provides the kernel access to information about the system's
                     //   RAM, layout and availability.
pub mod page_table;  // The page table management for the kernel.
pub mod mmu;         // Manage the MMU for the kernel.
pub mod heap;        // The kernel's heap allocator, including the implementation of the Rust global
                     //   allocator.
