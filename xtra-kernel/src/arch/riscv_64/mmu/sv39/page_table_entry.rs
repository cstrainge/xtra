
// Definition of the page table entry (PTE) as defined under the sv39 page table format
// specification.

use core::{ ops::{ Deref, Drop }, ptr::drop_in_place };

use crate::{ arch::mmu::{ PAGE_SIZE, sv39::{ page_table::PageTable } },
             memory::{ mmu::{ allocate_page, free_page } } };



/// These bits are reserved for future use and must be set to zero.
const PTE_RESERVED: u64
//          6            5           4            3           2            1           0
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_1111_1111_1100_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000;

/// Physical Page Number section 2.
const PTE_PPN_2: u64
//          6            5           4            3           2            1           0
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0011_1111_1111_1111_1111_1111_1111_0000_0000_0000_0000_0000_0000_0000;

/// Physical Page Number section 1.
const PTE_PPN_1: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_1111_1111_1000_0000_0000_0000_0000;

/// Physical Page Number section 0.
const PTE_PPN_0: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0111_1111_1100_0000_0000;

/// Reserved for software, these bits are defined by the OS.
const PTE_RSW: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0011_0000_0000;

/// Dirty bit, set if the page has been written to.
const PTE_D: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_1000_0000;

/// Accessed bit, set if the page has been read or written to.
const PTE_A: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0100_0000;

/// Global bit, set if the page entry is shared across all address spaces.
const PTE_G: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0010_0000;

/// User bit, it must be set for the page to be accessible in user mode.
const PTE_U: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0001_0000;

// Execute bit, it must be set for the page to be executable.
const PTE_X: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_1000;

/// Write bit, it must be set for the page table entry to be writable.
const PTE_W: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0100;

/// Read bit, it must be set for the page table entry to be readable.
const PTE_R: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0010;

/// Valid bit, it must always be set for the page table entry to be valid.
const PTE_V: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0001;



/// The page table entry structure for the SV39 page table format. The entry is a single 64-bit
/// value that contains the physical page number and various flags that control the access
/// permissions and attributes of the page.
#[repr(transparent)]
pub struct PageTableEntry(u64);



impl PageTableEntry
{
    /// Create a new page table entry, ready for setting up with the appropriate flags.
    pub const fn new() -> Self
    {
        PageTableEntry(0)
    }

    /// Create a new invalid page table entry.
    pub const fn new_invalid() -> Self
    {
        // Set the reserved bits and leave the valid bit unset.
        PageTableEntry(PTE_RESERVED)
    }

    /// Create a new page table entry that's a pointer to another page table.
    pub fn new_page_table_ptr() -> Self
    {
        let physical_address = allocate_page()
            .expect("Failed to allocate a page for the page table entry.");

        let mut entry = Self::new();

        entry.set_table_address(physical_address as *mut PageTable);
        entry
    }

    /// Change the state of a page table entry from invalid to valid but empty of any mappings.
    pub fn set_valid(&mut self)
    {
        // Make sure we're in a proper state.
        assert!(!self.is_valid(),
                "Cannot set a page table entry as valid when it is already valid.");

        // Set the valid bit and clear the reserved bits.
        self.0 |= PTE_V;
        self.0 &= !PTE_RESERVED;
    }

    /// Is this page table entry valid?
    pub fn is_valid(&self) -> bool
    {
           (self.0 & PTE_V) != 0
        && (self.0 & PTE_RESERVED) == 0
    }

    /// Mark this page table entry as invalid.
    pub fn set_invalid(&mut self)
    {
        // Check to see if this is an entry that points to a child page table. If it is we need to
        // free that page table as well.
        if self.is_page_table_ptr()
        {
            let page_table_ptr = self.get_table_address();
            let page_address = page_table_ptr as usize;

            unsafe
            {
                drop_in_place(page_table_ptr);
            }

            // Now free the memory that was allocated for the page table.
            free_page(page_address);
        }
        else if    self.is_leaf()
                && self.is_page_owned()
                && self.get_physical_address() != 0
        {
            // This entry contains a mapped page of RAM, check to see if we own the page, if we do
            // we can free it now.
            //
            // The reason for this check is that non-owned pages can be mapped into an address space
            // by the kernel. For example, shared memory regions or other kernel-managed pages.
            let physical_address = self.get_physical_address();
            free_page(physical_address);
        }

        // Clear all bits, including the valid bit.
        self.0 = 0;
    }

    /// Is the page table entry a pointer to another page table?
    pub fn is_page_table_ptr(&self) -> bool
    {
            self.is_valid()
        && !self.is_readable()
        && !self.is_writable()
        && !self.is_executable()
    }

    /// Is the entry a leaf entry? Meaning it refers to a page of RAM instead of another page table.
    pub fn is_leaf(&self) -> bool
    {
        self.is_valid() && !self.is_page_table_ptr()
    }

    /// Set this leaf entry as owning the page of RAM it refers to. This means when the page table
    /// that owns this entry is dropped the page of RAM will be freed automatically.
    pub fn set_page_owned(&mut self)
    {
        // This only makes sense for leaf entries.
        assert!(self.is_leaf(),
                "Cannot set page ownership on a page table entry that is not a leaf entry.");

        // We use the software reserved bits to indicate ownership of the page.
        self.0 |= (PTE_RSW & 1 << 8);
    }

    /// If the entry refers to a page, is the page owned by the table itself? If the page is owned
    /// then the page will automatically be freed when the table itself is dropped.
    pub fn is_page_owned(&self) -> bool
    {
        self.is_valid() && self.is_leaf() && ((self.0 & PTE_RSW) == 1)
    }

    /// Get the address of the page table this entry points to.
    ///
    /// This will panic if the entry is not a pointer to another page table.
    ///
    /// The address returned is the physical address of the page table, which is aligned to a
    /// page boundary (4096 bytes).
    pub fn get_table_address(&self) -> *mut PageTable
    {
        assert!(self.is_page_table_ptr(),
                "Page table entry is not a pointer to another page table.");

        // Extract the physical page number from the entry.
        let address = (((self.0 & (PTE_PPN_2 | PTE_PPN_1 | PTE_PPN_0)) >> 10) as usize) << 12;

        // Finally convert the raw address back to a pointer to a page table.
        address as *mut PageTable
    }

    /// Set this page table entry to point to another page table at the given address.
    ///
    /// This will panic if the address is not aligned to a page boundary (4096 bytes), or is too
    /// large for the SV39 page table format.
    fn set_table_address(&mut self, address: *mut PageTable)
    {
        // Convert the address to a usize for storing into the entry.
        let address = address as usize;

        // Ensure the address is aligned to a page boundary.
        assert!(address % PAGE_SIZE == 0,
                "Page table address {} is not aligned to a page boundary.",
                address);

        // Convert to page number.
        let address = (address >> 12) as u64;

        // A Sv39 PPN must fit in 44 bits
        assert!(address <= 0x003F_FFFF_FFFF,
               "Page table address {} is too large for Sv39.",
               address);

        // Clear the reserved bits and the access bits. The access bits are not valid when the entry
        // is a pointer to another page table.
        self.0 &= !PTE_RESERVED;
        self.0 &= !(PTE_PPN_2 | PTE_PPN_1 | PTE_PPN_0);

        // Encode into the 3 PPN sections of the page table entry.
        self.0 |= (address << 10) & (PTE_PPN_2 | PTE_PPN_1 | PTE_PPN_0);
    }

    /// Set the physical address of a page of RAM that this entry will refer to.
    pub fn set_physical_address(&mut self, physical_address: usize)
    {
        // Ensure the physical address is aligned to a page boundary.
        assert!(physical_address % PAGE_SIZE == 0,
                "Physical address {} is not aligned to a page boundary.",
                physical_address);

        // Convert to page number.
        let ppn = (physical_address >> 12) as u64;

        // A Sv39 PPN must fit in 44 bits
        assert!(ppn <= 0x003F_FFFF_FFFF,
               "Physical address {} is too large for Sv39.",
               physical_address);

        // Clear out the bits of the address first.
        self.0 &= !(PTE_PPN_2 | PTE_PPN_1 | PTE_PPN_0);

        // Now, encode the address into the 3 PPN sections of the page table entry.
        self.0 |= (ppn << 10) & (PTE_PPN_2 | PTE_PPN_1 | PTE_PPN_0);
    }

    /// Get a page of RAM's physical address from this page table entry.
    pub fn get_physical_address(&self) -> usize
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot get physical address from a page table entry that is a pointer to \
                another page table.");

        // Extract the physical page number from the entry.
        let ppn = (self.0 & (PTE_PPN_2 | PTE_PPN_1 | PTE_PPN_0)) >> 10;

        // Convert back to a physical address.
        (ppn as usize) << 12
    }

    /// Check to see if the page is dirty.
    pub fn is_dirty(&self) -> bool
    {
        (self.0 & PTE_D) != 0
    }

    /// Clear the dirty bit for this page.
    pub fn clear_dirty(&mut self)
    {
        self.0 &= !PTE_D;
    }


    /// Check if the page has been accessed.
    pub fn is_accessed(&self) -> bool
    {
        (self.0 & PTE_A) != 0
    }

    /// Clear the accessed bit for this page.
    pub fn clear_accessed(&mut self)
    {
        self.0 &= !PTE_A;
    }

    /// Set the page being referenced by this entry as global.
    pub fn set_global(&mut self, global: bool)
    {
        if global
        {
            self.0 |= PTE_G;
        }
        else
        {
            self.0 &= !PTE_G;
        }
    }

    /// Is the page being referenced by this entry global?
    pub fn is_global(&self) -> bool
    {
        (self.0 & PTE_G) != 0
    }


    // Set if the page being referenced by this entry is user accessible.
    pub fn set_user_accessible(&mut self, user_accessible: bool)
    {
        if user_accessible
        {
            self.0 |= PTE_U;
        }
        else
        {
            self.0 &= !PTE_U;
        }
    }

    /// Is the page being referenced by this entry user accessible?
    pub fn is_user_accessible(&self) -> bool
    {
        (self.0 & PTE_U) != 0
    }

    /// Set the page table entry's physical page number.
    pub fn set_ppn(&mut self, index: usize, ppn: usize)
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot set PPN on a page table entry that is a pointer to another page table.");

        match index
        {
            0 => self.0 = (self.0 & !PTE_PPN_0) | ((ppn as u64) << 10 & PTE_PPN_0),
            1 => self.0 = (self.0 & !PTE_PPN_1) | ((ppn as u64) << 19 & PTE_PPN_1),
            2 => self.0 = (self.0 & !PTE_PPN_2) | ((ppn as u64) << 28 & PTE_PPN_2),
            _ => panic!("Invalid PPN index {} for page table entry.", index)
        }
    }

    /// Set if the page being referenced by this entry is readable.
    pub fn set_readable(&mut self, readable: bool)
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot set readable on a page table entry that is a pointer to another page \
                table.");

        if readable
        {
            self.0 |= PTE_R;
        }
        else
        {
            self.0 &= !PTE_R;
        }
    }

    /// Is the page being referenced by this entry readable?
    pub fn is_readable(&self) -> bool
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot set writable on a page table entry that is a pointer to another page \
                table.");

        (self.0 & PTE_R) != 0
    }

    /// Set if the page being referenced by this entry is writable.
    pub fn set_writable(&mut self, writable: bool)
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot set writable on a page table entry that is a pointer to another page \
                table.");

        if writable
        {
            self.0 |= PTE_W;
        }
        else
        {
            self.0 &= !PTE_W;
        }
    }

    /// Is the page being referenced by this entry writable?
    pub fn is_writable(&self) -> bool
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot check writable on a page table entry that is a pointer to another page \
                table.");

        (self.0 & PTE_W) != 0
    }

    /// Set if the page being referenced by this entry is executable.
    pub fn set_executable(&mut self, executable: bool)
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot set executable on a page table entry that is a pointer to another page \
                table.");

        if executable
        {
            self.0 |= PTE_X;
        }
        else
        {
            self.0 &= !PTE_X;
        }
    }

    /// Is the page being referenced by this entry executable?
    pub fn is_executable(&self) -> bool
    {
        assert!(!self.is_page_table_ptr(),
                "Cannot check executable on a page table entry that is a pointer to another page \
                table.");

        (self.0 & PTE_X) != 0
    }
}



impl Default for PageTableEntry
{
    /// By default create an invalid page table entry.
    fn default() -> Self
    {
        Self::new_invalid()
    }
}



impl Deref for PageTableEntry
{
    type Target = u64;

    /// Give access to the raw page table entry data, which is a simple u64 value.
    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}



impl Drop for PageTableEntry
{
    /// Called on destruction, checks to see if the page table entry is in fact pointing at a
    /// sub-table. If we are pointing to a sub-table and not a leaf or invalid entry then we need to
    /// properly free that sub-table as well.
    fn drop(&mut self)
    {
        self.set_invalid();
    }
}
