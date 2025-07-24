
/*
/// Implementation of the low level memory manager for the RISC-V architecture.
use crate::memory::mmu::{ AddressSpace, HalMemoryManager, PageAccess, PageAllocCallback };



/// Implementation of the low level memory manager for the RISC-V architecture. This
/// implementation is based on the RISC-V MMU and provides a simple interface for allocating and
/// freeing pages of memory.
///
/// This will handle mapping physical memory to virtual addresses and managing the underlying page
/// tables.
///
/// This layer will also handle memory based interrupt handling and exceptions and will hand it off
/// to higher level abstractions as needed.
pub struct MemoryManager
{
}



impl MemoryManager
{
    /// Create a new blank instance of the memory manager. It will need to be properly initialized
    /// before it can be used.
    pub const fn new() -> Self
    {
        MemoryManager {}
    }
}



impl HalMemoryManager for MemoryManager
{
    /// Initialize the memory manager and the underlying MMU. This will setup the page tables and
    /// prepare the MMU for use.
    fn init(&mut self) -> Result<(), &'static str>
    {
        panic!("MMU initialization is not implemented yet.");
    }

    /// Allocate a new address space for a process or the kernel. This will call the callback to
    /// request a set of contiguous pages from the memory manager to store the address space
    /// bookkeeping information.
    ///
    /// This will return an error if the allocation fails.
    fn allocate_address_space(&mut self,
                              _callback: &mut PageAllocCallback)
                              -> Result<AddressSpace, &'static str>

    {
        Err("Address space allocation is not implemented yet.")
    }

    /// Map a physical page to a virtual address with the given access permissions. This will
    /// update the page tables to reflect the new mapping.
    ///
    /// This function also assumes that the memory subsystem is properly locked and that this will
    /// not be called recursively.
    fn map_page(&mut self,
                _address_space: &mut AddressSpace,
                _access: PageAccess,
                _virt_addr: usize,
                _phys_addr: usize) -> Result<(), &'static str>
    {
        Err("Mapping pages is not implemented yet.")
    }

    /// Unmap a virtual page, removing the mapping from the page tables. This will also clear the
    /// page table entry for the virtual address.
    ///
    /// This function will panic if the virtual address is not mapped or if the unmapping fails.
    ///
    /// This function also assumes that the memory subsystem is properly locked and that this will
    /// not be called recursively.
    fn unmap_page(&mut self,
                  _address_space: &mut AddressSpace,
                  _virt_addr: usize) -> Result<(), &'static str>
    {
        Err("Un-mapping pages is not implemented yet.")
    }
}



/// Get a reference to the global memory manager instance.
pub fn get_hal_memory_manager() -> &'static mut dyn HalMemoryManager
{
    static mut MEMORY_MANAGER: MemoryManager = MemoryManager::new();

    unsafe { &mut MEMORY_MANAGER }
}
*/