
// The implementation of an address space for the kernel or a user process. It is the address space
// that is used to manage the pages of memory available to a given process and how those pages are
// mapped into the virtual address space of that process.
//
// The address space also makes use of the higher level primitives provided by the MMU module to
// manage the pages of free memory in the system.

use crate::{ arch::mmu::{ page_table::{ PageManagement, PageTable } },
             locking::{ LockGuard, spin_lock::SpinLock },
             memory::{ mmu::{ allocate_page,
                              free_page,
                              get_kernel_memory_layout,
                              get_system_memory_layout,
                              page_box::PageBox,
                              permissions::Permissions,
                              physical_to_virtual_physical,
                              VIRTUAL_BASE_OFFSET },
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
        ///
        /// These are unmanaged pages, that is they are not allocated from the free page list but
        /// are special pages that are owned by the kernel itself.
        fn add_range(address_space: &mut AddressSpace,
                     physical_address: usize,
                     physical_range: usize,
                     permissions: Permissions,
                     virtualize_address: bool)
        {
            let base_address = physical_address;
            let end_address = physical_address + physical_range;

            for page_address in (base_address..end_address).step_by(PAGE_SIZE)
            {
                let virtual_address =
                    if virtualize_address
                    {
                        // If we are virtualizing the address then we need to add the base offset to
                        VIRTUAL_BASE_OFFSET + page_address
                    }
                    else
                    {
                        page_address
                    };

                address_space.page_table.map_page(virtual_address,
                                                  page_address,
                                                  permissions,
                                                  PageManagement::Manual)
                                        .expect("Failed to map page into address space");
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
                                                .build(),
                          false);
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
                                                .build(),
                          false);
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
                                        .build(),
                  false);

        // Map the kernel's read-only data section.
        add_range(&mut address_space,
                  kernel_memory.rodata.start,
                  kernel_memory.rodata.size,
                  Permissions::builder().readable()
                                        .globally_accessible()
                                        .build(),
                  false);

        // Map the kernel's data section.
        add_range(&mut address_space,
                  kernel_memory.data.start,
                  kernel_memory.data.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build(),
                  false);

        // Map the kernel's bss section.
        add_range(&mut address_space,
                  kernel_memory.bss.start,
                  kernel_memory.bss.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build(),
                  false);

        // Map the kernel's stack section.
        add_range(&mut address_space,
                  kernel_memory.stack.start,
                  kernel_memory.stack.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build(),
                  false);

        // Map the kernel's heap.
        add_range(&mut address_space,
                  kernel_memory.heap.start,
                  kernel_memory.heap.size,
                  Permissions::builder().readable()
                                        .writable()
                                        .globally_accessible()
                                        .build(),
                  false);

        // Map the kernel's virtual memory area. All physical pages of RAM will be mapped here so
        // that the kernel can access them directly.
        for device in get_system_memory_layout().memory_devices
        {
            if let Some(device) = device
            {
                add_range(&mut address_space,
                          device.base_address,
                          device.range,
                          Permissions::builder().readable()
                                                .writable()
                                                .globally_accessible()
                                                .build(),
                          true);
            }
        }

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
                         virtual_address: usize,
                         permissions: Permissions) -> Result<(), &'static str>
    {
        // Attempt to allocate a page of memory from the free page list. The free page list
        // maintains its own lock so we don't need to lock the address space yet.
        let page = allocate_page()
            .ok_or("Failed to allocate a page of memory from the free page list.")?;

        // Lock this address space
        let _guard = LockGuard::new(&self.lock);

        // Try to map the page into the address space at the given virtual address with the
        // given permissions. Mark the page as automatically managed so that it will be freed
        // back to the free page list when it is unmapped.
        let result = self.page_table
                         .map_page(virtual_address, page, permissions, PageManagement::Automatic);

        // If the mapping failed then we need to free the page back to the free page list so that
        // we don't leak the page.
        if result.is_err()
        {
            free_page(page);

            return Err("Failed to map page into address space.");
        }

        Ok(())
    }

    /// Free a page of memory at the given virtual address and return it back to the free page list.
    ///
    /// This will fail if the page at the given virtual address is not mapped.
    pub fn free_page(&mut self, virtual_address: usize) -> Result<(), &'static str>
    {
        // Try to unmap the page at the given virtual address. This will return the physical address
        // of the page if it isn't managed by the page table.
        let page =
            {
                // Lock the address space to ensure that we don't have multiple threads trying to
                // manage pages at the same time.
                let _guard = LockGuard::new(&self.lock);

                self.page_table.unmap_page(virtual_address)?
            };

        // Check if the page was owned by the page table.
        if let Some(page) = page
        {
            // If the page wasn't owned by the page table then need to free it now. The free page
            // list has it's own lock so we don't need to lock the address space again.
            free_page(page);
        }

        Ok(())
    }

    /// Map a specific page of memory into an address space at the given virtual address. It is
    /// assumed that the page is already allocated and is not part of the free page list.
    pub fn map_page(&mut self,
                    virtual_address: usize,
                    physical_address: usize,
                    permissions: Permissions) -> Result<(), &'static str>
    {
        // Lock the address space to ensure that we don't have multiple threads trying to manage
        // pages at the same time.
        let _guard = LockGuard::new(&self.lock);

        // Attempt to map the page into the address space at the given virtual address with the
        // given permissions. Mark the page as manually managed so that it will not be freed back
        // to the free page list when it is unmapped.
        self.page_table.map_page(virtual_address,
                                 physical_address,
                                 permissions,
                                 PageManagement::Manual)
    }

    /// Unmap a page of memory at the given virtual address. This will remove the mapping from the
    /// address space. The free page list will remain untouched.
    pub fn unmap_page(&mut self, virtual_address: usize) -> Result<usize, &'static str>
    {
        // Lock the address space to ensure that we don't have multiple threads trying to manage
        // pages at the same time.
        let _guard = LockGuard::new(&self.lock);

        let page = self.page_table.unmap_page(virtual_address)?;

        // If we didn't get an address back then the page was owned by the page table.
        assert!(page.is_some(),
                "The page at virtual address {} was not managed by the page table.",
                virtual_address);

        // Return the physical address of the page that was unmapped.
        Ok(page.unwrap())
    }

    /// Given a virtual address find the physical address that the virtual address represents.
    ///
    /// Will return an error if the virtual address is not mapped in the address space.
    pub fn get_physical_address(&self, virtual_address: usize) -> Result<usize, &'static str>
    {
        // Lock the address space to ensure that we don't have multiple threads trying to manage
        // pages at the same time.
        let _guard = LockGuard::new(&self.lock);

        // Attempt to look up the physical address for the given virtual address in the page table.
        let physical_address = self.page_table.get_physical_address(virtual_address)?;

        // Return the physical address that the virtual address represents.
        Ok(physical_address)
    }
}
