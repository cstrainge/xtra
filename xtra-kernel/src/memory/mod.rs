
// The memory module provides the kernel with the ability to manage memory, including allocating
// and freeing memory as well as managing the virtual memory space of the kernel and user processes.



// TODO: Make this a kernel configuration option so that we can change the page size at compile
//       time.

/// The size of a memory page as used on this system/architecture. This is typically 4KB on most
/// systems, but can vary based on the architecture and configuration.
const PAGE_SIZE: usize = 4096;  // Our memory is split into 4KB pages.



// The sub-modules of the memory subsystem.

/// Information about the kernel's memory usage and layout.
pub mod kernel;


/// The RAM device provides the kernel access to information about the system's RAM, layout and
/// availability.
pub mod memory_device;


/// The high level MMU interface for the kernel. The interface is architecture agnostic and will
/// manage the pages of memory in the system, both used and free.
pub mod mmu;


/// Our Rust heap and global allocator are implemented in this module. It is built on top of the
/// MMU module and provides the interface for allocating and freeing memory from the heap.
pub mod heap;
