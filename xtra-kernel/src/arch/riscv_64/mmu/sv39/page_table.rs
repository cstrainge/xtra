
// Implementation of the page table as defined under the sv39 page table format specification.
//
// This code specifically does not use the heap due to requirements of the RISC-V 64-bit
// architecture and the fact that the page table is a fixed size structure that is always allocated
// at page table aligned addresses.
//
// This implementation of the page table only supports allocating 4KB pages.
//
// This code also assumes that the physical pages of RAM have been allocated from the system's free
// page pool and are available for use. It does not check to see if the RAM pointed to is valid.
//
// The page table also supports iterating over all the allocated pages in the page table, skipping
// all invalid or empty entries in the page table(s).

use core::{ fmt::Write, mem::size_of };

use crate::{ arch::mmu::{ PAGE_SIZE,
                          sv39::{ page_table_entry::PageTableEntry,
                                  virtual_address::VirtualAddress } },
             printing::BufferWriter,
             memory::{ mmu::{ page_box::PageBoxable,
                              permissions::Permissions,
                              virtual_page_ptr::VirtualPagePtr } } };



/// Reexport the PageManagement enum so that it can be used by users of the PageTable.
pub use crate::arch::mmu::sv39::page_table_entry::PageManagement;



/// The maximum number of entries in a page table is 512, as defined by the RISC-V SV39
/// specification. Each entry is 8 bytes, so the total size of a page table is
/// 512 * 8 = 4096 bytes (4KB), which is the standard page size for RISC-V 64-bit systems.
pub const PAGE_TABLE_SIZE: usize = 512;



/// The maximum number of levels of indirection in a page table is 3, as defined by the RISC-V SV39
/// specification.
const MAX_TABLE_INDIRECTIONS: usize = 3;



/// The page table structure for the SV39 page table format. It contains an array of 512
/// `PageTableEntry` entries, each of which is 8 bytes in size. The total size of the page table
/// is 4096 bytes (4KB), which is the standard page size for RISC-V 64-bit systems.
///
/// It is the job of the page table to manage the mapping of virtual addresses to physical addresses
/// and to provide the necessary functions to manipulate these mappings.  Ie, converting a virtual
/// address to a physical address, setting and clearing page table entries, etc.
///
/// A page table lookup can be up to 3 levels deep, with a root page table that points to a second
/// level page table, which in turn points to a third level page table. Each level of the page table
/// can have up to 512 entries, allowing for a large address space to be mapped.
#[repr(C, align(4096))]
pub struct PageTable
{
    entries: [PageTableEntry; PAGE_TABLE_SIZE]
}



/// Ensure that the size of the page table is exactly 4096 bytes (4KB), as required by the RISC-V
/// SV39 specification.
const _: () =
    {
        assert!(size_of::<PageTable>() == PAGE_SIZE,
                "The size of the page table must be 4096 bytes (4KB).");
    };



impl PageTable
{
    /// Internal function to convert a raw page address into a mutable reference to a
    /// `PageTable`.
    ///
    /// This function assumes the address actually references a valid page of memory that is
    /// available for use.
    ///
    /// It will panic if the address is not aligned to the page size or if the page is not
    /// properly initialized.
    pub unsafe fn from_physical_address(page_address: usize) -> *mut Self
    {
        assert!((page_address % PAGE_SIZE) == 0,
                "Page address must be aligned to the page size ({} bytes).",
                PAGE_SIZE);

        let page_table = page_address as *mut Self;

        for entry in unsafe { &mut (*page_table).entries }
        {
            let address = entry as *const PageTableEntry as usize;
            let new_value = PageTableEntry::new_invalid();
            *entry = new_value;
        }

        page_table
    }

    /// Get an immutable iterator for all of the pages mapped in the page table.
    // pub fn iter(&self) -> PageTableIterator<'_>
    // {
    //     PageTableIterator::new(self)
    // }


    /// Map a physical page of RAM into an address space at the given virtual address.
    pub fn map_page(&mut self,
                    virtual_address: usize,
                    physical_address: usize,
                    permissions: Permissions,
                    page_management: PageManagement) -> Result<(), &'static str>
    {
        unsafe
        {
            // Convert the raw virtual address into a proper virtual address so that we can access
            // it's fields.
            let virtual_address = VirtualAddress::new(virtual_address);

            // Make sure that the virtual and physical addresses are aligned and non-zero.
            if virtual_address.get_offset() != 0
            {
                return Err("Virtual address must be page aligned.");
            }

            if    physical_address % PAGE_SIZE != 0
               || physical_address == 0
            {
                return Err("Physical address must be page aligned and non-zero.");
            }

            // Look up the page table entry in the third level table.
            let entry = &mut self.look_up_page_entry_mut(&virtual_address)?;

            // If the entry is already valid then this page has already been mapped so we return an
            // error at this point.
            if entry.is_valid()
            {
                return Err("The page has already been mapped.");
            }

            // Reset the entry from being invalid to a leaf entry.
            entry.set_valid();

            // Make sure the last access bits are cleared.
            entry.clear_accessed();
            entry.clear_dirty();

            // Translate the permission flags into the proper permission bits in the page table
            // entry.
            entry.set_global(permissions.globally_accessible);
            entry.set_user_accessible(permissions.user_accessible);
            entry.set_readable(permissions.readable);
            entry.set_writable(permissions.writable);
            entry.set_executable(permissions.executable);
            entry.set_page_management(page_management);

            // Finally set the page's physical address in the page table entry.
            entry.set_physical_address(physical_address);
        }

        Ok(())
    }

    /// Forcibly unmap a page from the page table at the given virtual address.
    ///
    /// If the pointed to page was manually managed then we will return the physical address of
    /// the page that was unmapped, otherwise we will return `None` to indicate that the page was
    /// automatically managed and we do not return the physical address.
    ///
    /// If the page was CopyOnWrite then we will not return the physical address either because it
    /// is assumed that the page is owned by another process.
    pub fn unmap_page(&mut self, virtual_address: usize) -> Result<Option<usize>, &'static str>
    {
        // Convert the raw virtual address into a proper virtual address so that we can access
        // it's fields.
        let virtual_address = VirtualAddress::new(virtual_address);

        // Make sure that the virtual and physical addresses are aligned and non-zero.
        if virtual_address.get_offset() != 0
        {
            return Err("Virtual address must be page aligned and non-zero.");
        }

        // Look up the page table entry in the third level table.
        let entry = self.look_up_page_entry_mut(&virtual_address)?;

        // If the page isn't owned by the page table, we don't free it, but we can return it's
        // address.
        let freed_page = match entry.get_page_management()
            {
                PageManagement::Manual      => Some(entry.get_physical_address()),
                PageManagement::Automatic   => None,
                PageManagement::CopyOnWrite => None,
                PageManagement::CowOwner    => None
            };

        // Set the entry to be invalid which will also clear the physical address and permissions.
        // This will automatically free any associated memory as needed.
        entry.set_invalid();

        // All done, return the freed page if it wasn't owned by the page table.
        Ok(freed_page)
    }

    /// Attempt to look up the physical address for a given virtual address in the page table.
    ///
    /// Will return an error if the virtual address is not mapped in the page table, or if the
    /// page table entry is not a leaf entry.
    pub fn get_physical_address(&self, virtual_address: usize) -> Result<usize, &'static str>
    {
        // Convert the raw virtual address into a proper virtual address so that we can access
        // it's fields.
        let virtual_address = VirtualAddress::new(virtual_address);

        // Look up the page table entry in the third level table.
        let entry = self.look_up_page_entry(&virtual_address)?;

        // Make sure that the entry refers to a physical address.
        if !entry.is_leaf()
        {
            return Err("The page table entry is not a leaf entry, it is a page table pointer.");
        }

        // Ok, translate the virtual address to the physical address.
        let base_physical_address = entry.get_physical_address();

        Ok(base_physical_address + virtual_address.get_offset())
    }

    /// Given a virtual address look up a page table entry for that address.
    ///
    /// There may or may not be a page of RAM mapped by that entry.
    fn look_up_page_entry_mut(&mut self,
                              virtual_address: &VirtualAddress)
                              -> Result<&mut PageTableEntry, &'static str>
    {
        // Look up the page table entry for the given virtual address. This is a three level lookup
        // because we only support allocating 4k pages. In other implementations of the page table
        // we could support larger pages, and in that case we'd need to check to see if the search
        // should stop at a higher order page table.
        let vpn2 = virtual_address.get_vpn(2);
        let vpn1 = virtual_address.get_vpn(1);
        let vpn0 = virtual_address.get_vpn(0);

        unsafe
        {
            // Get the second level page table.
            let mut second_level_table = if self.entries[vpn2].is_valid()
                {
                    if !self.entries[vpn2].is_page_table_ptr()
                    {
                        return Err("The entry at VPN[2] must be a page table pointer.");
                    }

                    self.entries[vpn2].get_table_address()
                }
                else
                {
                    self.entries[vpn2] = PageTableEntry::new_page_table_ptr();
                    self.entries[vpn2].get_table_address()
                };

            // Look up the third level table from the second level table.
            let mut third_level_table = if second_level_table.entries[vpn1].is_valid()
            {
                    if !(*second_level_table).entries[vpn1].is_page_table_ptr()
                    {
                        return Err("The entry at VPN[1] must be a page table pointer.");
                    }

                    (*second_level_table).entries[vpn1].get_table_address()
                }
                else
                {
                    second_level_table.entries[vpn1] = PageTableEntry::new_page_table_ptr();
                    second_level_table.entries[vpn1].get_table_address()
                };

                // Look up the page table entry in the third level table.
            let raw_ptr = &mut third_level_table.entries[vpn0] as *mut PageTableEntry;

            Ok(&mut *raw_ptr)
        }
    }

    /// Given a virtual address look up a page table entry for that address.
    ///
    /// There may or may not be a page of RAM mapped by that entry.
    fn look_up_page_entry(&self,
                          virtual_address: &VirtualAddress)
                          -> Result<&PageTableEntry, &'static str>
    {
        // Look up the page table entry for the given virtual address. This is a three level lookup
        // because we only support allocating 4k pages. In other implementations of the page table
        // we could support larger pages, and in that case we'd need to check to see if the search
        // should stop at a higher order page table.
        let vpn2 = virtual_address.get_vpn(2);
        let vpn1 = virtual_address.get_vpn(1);
        let vpn0 = virtual_address.get_vpn(0);

        unsafe
        {
            // Get the second level page table.
            let second_level_table = if self.entries[vpn2].is_valid()
                {
                    if !self.entries[vpn2].is_page_table_ptr()
                    {
                        return Err("The entry at VPN[2] must be a page table pointer.");
                    }

                    self.entries[vpn2].get_table_address()
                }
                else
                {
                    return Err("The entry at VPN[2] is not a valid page table pointer.");
                };

            // Look up the third level table from the second level table.
            let third_level_table = if (*second_level_table).entries[vpn1].is_valid()
                {
                    if !(*second_level_table).entries[vpn1].is_page_table_ptr()
                    {
                        return Err("The entry at VPN[1] must be a page table pointer.");
                    }

                    (*second_level_table).entries[vpn1].get_table_address()
                }
                else
                {
                    return Err("The entry at VPN[1] is not a valid page table pointer.");
                };

            // Look up the page table entry in the third level table.
            let raw_ptr = &third_level_table.entries[vpn0] as *const PageTableEntry;

            Ok(&*raw_ptr)
        }
    }
}



impl PageBoxable for PageTable
{
    /// Allow the page table to be constructed directly from a page of memory without needing to
    /// allocate a new page.
    unsafe fn init_in_place(page_address: &mut VirtualPagePtr<Self>)
    {
        unsafe
        {
            Self::from_physical_address(page_address.as_usize());
        }
    }
}
