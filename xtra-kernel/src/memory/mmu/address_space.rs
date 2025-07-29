
// The implementation of an address space for the kernel or a user process. It is the address space
// that is used to manage the pages of memory available to a given process and how those pages are
// mapped into the virtual address space of that process.
//
// The address space also makes use of the higher level primitives provided by the MMU module to
// manage the pages of free memory in the system.

use crate::{ arch::mmu::page_table::PageTable,
             locking::spin_lock::SpinLock,
             memory::{ mmu::{ get_kernel_memory_layout,
                              get_system_memory_layout,
                              page_box::PageBox,
                              permissions::Permissions },
                     PAGE_SIZE } };



/// High-level representation of the an address space for the kernel or a user process.
pub struct AddressSpace
{
    /// The page table for this address space. This is defined in the architecture specific module
    /// for actual MMU used by the CPU.
    page_table: PageBox<PageTable>,

    /// A lock to ensure that the address space is not modified by multiple threads at the same
    /// time. We're trying to avoid a global lock for all address spaces so that processes on
    /// separate cores can allocate memory in parallel.
    lock: SpinLock
}



impl AddressSpace
{
    /// Construct a new address space for the kernel or a user process. Creates a new address space
    /// with the kernel and several devices mapped into it.
    pub fn new() -> Self
    {
        /// Break up a range of physical memory into pages and map them into the address space with
        /// the given permissions.
        fn add_range(address_space: &mut AddressSpace,
                     physical_address: usize,
                     physical_range: usize,
                     permissions: Permissions)
        {
            let base_address = physical_address;
            let end_address = physical_address + physical_range;

            for page_address in (base_address..end_address).step_by(PAGE_SIZE)
            {
                address_space.map_page(page_address, page_address, permissions)
                             .expect("Failed to map page into address space.");
            }
        }

        // Allocate a page table for the address space and init the spin lock for the address space.
        let mut address_space = AddressSpace
            {
                page_table: PageBox::<PageTable>::new(),
                lock: SpinLock::new()
            };

        // Get the system and kernel memory layouts.
        let kernel_memory = get_kernel_memory_layout();
        let system_memory = get_system_memory_layout();

        // Map the flash devices with read only permission for the kernel.
        // TODO: Plan for supporting writable flash devices in the future.
        for device in system_memory.flash_devices
        {
            if let Some(device) = device
            {
                add_range(&mut address_space,
                          device.base_address,
                          device.range,
                          Permissions::builder().readable()
                                                .globally_accessible()
                                                .build());
            }
        }

        // We then have to map all MMIO pages into the address space with read and write access for
        // the kernel only.
        for region in system_memory.mmio_regions
        {
            if let Some(region) = region
            {
                add_range(&mut address_space,
                          region.base_address,
                          region.range,
                          Permissions::builder().readable()
                                                .writable()
                                                .globally_accessible()
                                                .build());
            }
        }

        // Map the kernel's memory pages into the address space with the permissions that make sense
        // for each section of the kernel.

        // Start with the kernel's code section.
        add_range(&mut address_space,
                  kernel_memory.text.start,
                  kernel_memory.text.size,
                  Permissions::builder().readable()
                                        .executable()
                                        .globally_accessible()
                                        .build());

        // Map the kernel's read-only data section.
        add_range(&mut address_space,
                  kernel_memory.rodata.start,
                  kernel_memory.rodata.size,
                  Permissions::builder().readable()
                                        .globally_accessible()
                                        .build());

        // Map the kernel's data section.
        add_range(&mut address_space,
                  kernel_memory.data.start,
                  kernel_memory.data.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build());

        // Map the kernel's bss section.
        add_range(&mut address_space,
                  kernel_memory.bss.start,
                  kernel_memory.bss.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build());

        // Map the kernel's stack section.
        add_range(&mut address_space,
                  kernel_memory.stack.start,
                  kernel_memory.stack.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build());

        // Map the kernel's heap.
        add_range(&mut address_space,
                  kernel_memory.heap.start,
                  kernel_memory.heap.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build());

        // Now that we have the common regions of memory mapped out we can leave the rest of the
        // address space as free pages.
        address_space
    }

    /// Make this address space the current address space for the current core.
    pub fn make_current(&self)
    {
        // Switch the MMU to use this address space.
    }

    /// Allocate a page of memory from the free list and map it into an address space at the given
    /// virtual address and permissions.
    ///
    /// This will either allocate and map the page or return an error if the page could not be
    /// allocated or mapped for some reason.
    pub fn allocate_page(&mut self,
                         _virtual_address: usize,
                         _permissions: Permissions) -> Result<usize, &'static str>
    {
        Err("Unimplemented: allocate_page in AddressSpace.")
    }

    /// Free a page of memory at the given virtual address and return it back to the free page list.
    ///
    /// This will fail if the page at the given virtual address is not mapped.
    pub fn free_page(&mut self, _virtual_address: usize) -> Result<(), &'static str>
    {
        // Unmap the page at the given virtual address and free it back to the kernel's memory manager.
        Err("Unimplemented: free_page in AddressSpace.")
    }

    /// Map a specific page of memory into an address space at the given virtual address. It is
    /// assumed that the page is already allocated and is not part of the free page list.
    pub fn map_page(&mut self,
                    _virtual_address: usize,
                    _physical_address: usize,
                    _permissions: Permissions) -> Result<(), &'static str>
    {
        Err("Unimplemented: map_page in AddressSpace.")
    }

    /// Unmap a page of memory at the given virtual address. This will remove the mapping from the
    /// address space. The free page list will remain untouched.
    pub fn unmap_page(&mut self, _virtual_address: usize) -> Result<(), &'static str>
    {
        Err("Unimplemented: unmap_page in AddressSpace.")
    }

    /// Given a virtual address find the physical address that the virtual address represents.
    ///
    /// Will return an error if the virtual address is not mapped in the address space.
    pub fn get_physical_address(&self, _virtual_address: usize) -> Result<usize, &'static str>
    {
        Err("Unimplemented: get_physical_address in AddressSpace.")
    }
}
