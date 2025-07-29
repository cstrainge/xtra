
// High level memory page management for the kernel. It is here that we define how pages of memory
// are allocated and freed, how the MMU is initialized and how the kernel's/user's address spaces
// are set up.
//
// The low level MMU hardware interactions are handled by the mmu sub-modules of the arch module.
// All hardware independent code lives in this module and the sub-modules of the memory module.
//
// We don't make use of a heap allocator in this code. This is because the heap will be built on top
// of the MMU code and having circular dependencies can be highly problematic. Instead, we implement
// structures like the `PageBox` that functions like a box but works directly with pages of memory.

use crate::{ locking::{ LockGuard, spin_lock::SpinLock },
             memory::{ //mmu::page_box::{ PageBox, PageBoxable },
                       kernel::KernelMemoryLayout,
                       memory_device::SystemMemory } };



/// Internal module for managing the list of free memory pages in the system.
mod free_page_list;


/// The permissions that can be applied to a page of memory when it is mapped into an address space.
pub mod permissions;


/// The high-level representation of a virtual address space in the kernel. This is used for both
/// user processes and the kernel itself.
///
/// It provides the methods of allocating and freeing pages of memory of an address space and where
/// in that address space those pages are mapped.
pub mod address_space;


/// Implementation of a box that works directly with pages of memory. That is a page is both the
/// smallest the size of memory that can be allocated and the maximum size of the contained type.
///
/// The page box is also designed to allow the contained type to be constructed directly from the
/// allocated memory instead of needing to allocate its data on the stack first.
pub mod page_box;



use free_page_list::init_free_page_list;

use address_space::{ AddressSpace };



/// The global kernel address space. This is the address space that is used by the kernel and idle
/// process. It is initialized during the kernel's boot process.
static mut KERNEL_ADDRESS_SPACE: Option<AddressSpace> = None;



/// A global lock to protect access to the free page list. It can be accesses at any time from any
/// thread context. So we need to ensure that it is protected from concurrent access.
static FREE_PAGE_LOCK: SpinLock = SpinLock::new();



static mut KERNEL_MEMORY: KernelMemoryLayout = KernelMemoryLayout::zeroed();



static mut SYSTEM_MEMORY: Option<SystemMemory> = None;



/// Initialize the system's memory management unit, (MMU,) and the higher level data strictures
/// around it.
///
/// The page tables for the kernel itself will be initialized and the free page list allocated and
/// prepared for allocating pages of memory for the kernel and user processes.
///
/// We also zero initialize all the free pages in the system so that we can safely use them for any
/// purpose.
pub fn init_memory_manager(kernel_memory: &KernelMemoryLayout,
                           system_memory: &SystemMemory) -> Result<(), &'static str>
{
    // Initialize the free page list. Now we will be able to keep track of the free pages in the
    // system. We make sure to not allocate any pages that are part of the kernel's memory layout
    // and also avoid allocating pages that belong to MMIO devices.
    init_free_page_list(kernel_memory, system_memory);

    // keep copies of the kernel and system memory layouts for later use.
    unsafe
    {
        KERNEL_MEMORY = *kernel_memory;
        SYSTEM_MEMORY = Some(*system_memory);
    }

    // Create the kernel's address space.

    Ok(())
}



/// Get the kernel's memory layout. This represents where the kernel is loaded in physical memory
/// and how it's internal sections are laid out.
pub fn get_kernel_memory_layout() -> KernelMemoryLayout
{
    unsafe
    {
        //KERNEL_MEMORY.as_ref().expect("Kernel memory layout not initialized.")
        KERNEL_MEMORY
    }
}



/// Get the layout of the system's memory. This includes the physical RAM layout, MMIO regions and
/// other memory regions.
pub fn get_system_memory_layout() -> SystemMemory
{
    unsafe
    {
        SYSTEM_MEMORY.expect("System memory layout not initialized.")
    }
}



/// This function will switch from the raw address space to the virtual address space of the kernel
/// this will map the kernel into high memory and switch the MMU to use the kernel's page tables as
/// initialized earlier by the memory manager's initialization function.
///
/// This will also adjust the kernel's stack pointer and reset the PC to the new virtual address
/// space of the kernel.
///
/// THis function will panic on failure.
pub fn convert_to_kernel_address_space()
{
    // Switch the MMU to use the kernel's address space.

    panic!("Switching to kernel address space is not implemented yet.");
}



/// Allocate a page of memory from the free page list and return the physical address of the page.
///
/// This function will return `None` if no pages are available for allocation.
///
/// This function should not be confused with the `allocate_page` function in the `AddressSpace`
/// struct. This function is used to allocate a page of memory from the global free page list but
/// does not manage mapping the page into an address space.
///
/// This function is used to allocate pages of memory for the kernel's internal data structures.
pub fn allocate_page() -> Option<usize>
{
    let _guard = LockGuard::new(&FREE_PAGE_LOCK);

    free_page_list::remove_free_page()
}



/// Free a page of physical memory and return it back to the free page list.
///
/// This will panic if the page is already in the free page list or if the address is not a valid
/// page address.
///
/// This function is used to free pages of memory that were allocated by the `allocate_page`
/// function.
///
/// This function should not be confused with the `free_page` function in the `AddressSpace` struct.
/// This function is used to free pages of memory that were allocated for the kernel's internal
/// data structures. If you wish to free a page of memory from an address space you should use the
/// appropriate method on the `AddressSpace` struct.
pub fn free_page(physical_page_address: usize)
{
    let _guard = LockGuard::new(&FREE_PAGE_LOCK);

    free_page_list::add_free_page(physical_page_address);
}



/// Attempt to allocate a set of contiguous pages of physical memory and return the physical
/// address of the first page in the set.
///
/// If there are no pages available for allocation or the requested number of pages is can not be
/// contiguously allocated, then this function will return `None`.
///
/// Otherwise the physical address of the first page in the set will be returned.
pub fn allocate_n_pages(count: usize) -> Option<usize>
{
    let _guard = LockGuard::new(&FREE_PAGE_LOCK);

    free_page_list::remove_n_free_pages(count)
}



/// Free a set of contiguous pages of physical memory and return them back to the free page list for
/// later reallocation.
pub fn free_n_pages(physical_page_address: usize, count: usize)
{
    let _guard = LockGuard::new(&FREE_PAGE_LOCK);

    free_page_list::add_n_free_pages(physical_page_address, count);
}
