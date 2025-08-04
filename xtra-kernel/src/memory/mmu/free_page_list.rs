
/// Module that manages the list of free memory pages in the system. The unused pages need to be
/// properly kept track of so that they can be reused as needed. The pages in use should be kept
/// track of by the page table system used on the current architecture.
///
/// This module provides an architecture agnostic way of managing the free pages in the system.
///
/// Note: This module doesn't lock itself, it is up to the higher level MMU module to ensure that
/// all accesses to this code is thread safe as the free page list will be shared across all cores
/// in the system.
///
/// Note that because this code lives below the heap, it can not make use of the heap for any memory
/// allocations. This means that the free page list is intrusive and lives within the pages it
/// manages. This is a low level module and should be used with care.

use core::{ mem::size_of, slice::from_raw_parts_mut };

use crate::memory::{ PAGE_SIZE,
                     kernel::KernelMemoryLayout,
                     memory_device::SystemMemory,
                     mmu::virtual_page_ptr::VirtualPagePtr };



/// The bookkeeping for the free pages are kept within the page itself because that memory isn't
/// being used for anything else, and so that frees up any constraints on how many free pages we can
/// keep track of at any given time.
#[derive(Clone, Copy, PartialEq, Eq)]
struct FreeMemoryPage
{
    /// Physical address of the page. The pointer to the page and this address should be the same.
    pub address: usize,

    /// The previous page in the list, if any.
    pub prev_page: Option<FreeMemoryPagePtr>,

    /// The next page in the list, if any.
    pub next_page: Option<FreeMemoryPagePtr>
}



/// Pointer to a free memory page.
type FreeMemoryPagePtr = VirtualPagePtr<FreeMemoryPage>;



impl FreeMemoryPage
{
    /// Create a new memory page structure at the given address with the given previous and next
    /// pages.
    ///
    /// This is potentially unsafe because it assumes that the page address is valid and that the
    /// previous and next pages are valid pointers to FreeMemoryPage structures.
    ///
    /// Needless to say, this is a low level operation and should be used with care. Care must be
    /// taken to ensure that the address is the proper start of a page in memory and that the page
    /// is actually free.
    pub fn new(address: usize,
               prev_page: Option<FreeMemoryPagePtr>,
               next_page: Option<FreeMemoryPagePtr>) -> FreeMemoryPagePtr
    {
        // Make sure that the page address makes sense. And the base integer size is aligned to the
        // page size. We also make sure that our book-keeping structure will safely fit within a
        // given page.
        assert!(address % PAGE_SIZE == 0,
                "Address must be aligned to page boundary, got 0x{:x} instead.",
                address);

        assert!(PAGE_SIZE % size_of::<usize>() == 0,
                "PAGE_SIZE must be a multiple of usize size, got {} instead.",
                PAGE_SIZE);

        assert!(PAGE_SIZE >= size_of::<FreeMemoryPage>(),
                "PAGE_SIZE must be at least as large as FreeMemoryPage size, ({},) got {} instead.",
                size_of::<FreeMemoryPage>(),
                PAGE_SIZE);

        // Zero out the page to ensure that it is clean and ready for use. We use native word size
        // writes to zero out the page. This is more efficient than writing byte by byte. Also many
        // systems don't allow misaligned writes so this avoids the compiler generating a lot of
        // extra code to simulate writing individual bytes.

        let page_slice = unsafe { from_raw_parts_mut(address as *mut usize,
                                                     PAGE_SIZE / size_of::<usize>()) };

        for chunk in page_slice.iter_mut()
        {
            *chunk = 0;
        }

        // Get a pointer to the new page stricture within the page itself.  Then we can create the
        // FreeMemoryPage structure at that address.
        let page_ptr = FreeMemoryPagePtr::try_from(address);

        if let Err(e) = page_ptr
        {
            panic!("Failed to create FreeMemoryPagePtr from address 0x{:x}: {}", address, e);
        }

        let mut page_ptr = page_ptr.unwrap();

        *page_ptr = FreeMemoryPage
            {
                address: page_ptr.as_physical_address(),
                prev_page,
                next_page
            };

        // Return the pointer to the new page.
        page_ptr
    }

    /// Clear out our internal bookkeeping for a page. This we we don't have stale pointers and we
    /// don't leak internal data to other systems.
    pub fn clear(&mut self)
    {
        self.address = 0;
        self.prev_page = None;
        self.next_page = None;
    }
}



/// Representation of all of the unused pages of RAM in the system. It is an intrusive doubly linked
/// list of FreeMemoryPage structures. The structures will live within the pages themselves, so the
/// only overhead is the size of this structure itself.
///
/// We are going with a doubly linked list so that we can efficiently add and remove pages from the
/// list inside of the list, which will be useful when we need to allocate or free bulk sets of
/// pages at a time.
///
/// In the future we may want to evolve this to a more complex data structure, such as a tree or a
/// buddy allocator. But for this phase of the kernel we are going with a simpler implementation.
struct FreePageList
{
    /// The first page in the list.
    pub first_page: Option<FreeMemoryPagePtr>,

    /// The last page in the list.
    pub last_page: Option<FreeMemoryPagePtr>
}



impl FreePageList
{
    /// Create a new empty free page list.
    pub const fn new() -> Self
    {
        FreePageList { first_page: None, last_page: None }
    }


    /// Insert a new page into the free page list at the beginning of the list.
    ///
    /// It is a fatal error if the new page is not logically before the first page in the list. (If
    /// any.)
    pub fn add_free_page_to_beginning(&mut self, mut page: FreeMemoryPagePtr)
    {
        if self.is_empty()
        {
            self.first_page = Some(page);
            self.last_page = Some(page);
        }
        else
        {
            let mut first_page_ptr = self.first_page.unwrap();

            // Validate that the first page pointer is not null and that it doesn't have a
            // previous page. Also make sure that we are properly adding the new page before the
            // first page both in the list and in the logical address space.
            assert!(first_page_ptr.prev_page.is_none(),
                    "First page pointer must not have a previous page when adding a new page.");

            assert!(page.address > first_page_ptr.address,
                    "New page address must be greater than the first page address. \
                    Trying to add page at 0x{:x} before first page at 0x{:x}.",
                    page.address,
                    first_page_ptr.address);

            page.next_page = Some(first_page_ptr);
            first_page_ptr.prev_page = Some(page);

            self.first_page = Some(page);
        }
    }

    /// Add a free page to the end of the free page list.
    pub fn add_free_page_to_end(&mut self, mut page: FreeMemoryPagePtr)
    {
        // If the list is empty, then this is the first page.
        if self.is_empty()
        {
            self.first_page = Some(page);
            self.last_page = Some(page);
        }
        else
        {
            // Otherwise, we need to add it to the end of the list.
            let mut last_page_ptr = self.last_page.unwrap();

            // Validate that the last page pointer is not null and that it doesn't have a next
            // page. Also make sure that we are properly adding the new page after the last page
            // both in the list and in the logical address space.
            //
            // One of the key requirements of this free page list is that it is properly sorted
            // by address and that contiguous pages are added in order. This is to ensure that
            // we can efficiently allocate and free pages in bulk without having to worry about
            // gaps in the address space.
            assert!(last_page_ptr.next_page.is_none(),
                    "Last page pointer must not have a next page when adding a new page.");

            assert!(last_page_ptr.address < page.address,
                    "New page address must be greater than the last page address. \
                    Trying to add page at 0x{:x} after last page at 0x{:x}.",
                    page.address,
                    last_page_ptr.address);

            last_page_ptr.next_page = Some(page);

            page.prev_page = Some(last_page_ptr);
            self.last_page = Some(page);
        }
    }

    /// Insert a free page into the free page list. This will insert the page in the correct
    /// position in the list based on its address.
    pub fn insert_page(&mut self, mut new_page: FreeMemoryPagePtr)
    {
        // If the list is empty then just add the page to the end of the list.
        if self.is_empty()
        {
            self.add_free_page_to_end(new_page);
            return;
        }

        // The list isn't empty, so check if the page is after the end of the list saving us a
        // search.
        if self.is_page_at_end(new_page)
        {
            self.add_free_page_to_end(new_page);
            return;
        }

        // Does the new page belong at the beginning of the list? If so, we can add it directly to
        // the beginning of the list without searching for a parent page.
        if self.is_page_at_beginning(new_page)
        {
            self.add_free_page_to_beginning(new_page);
            return;
        }

        // The new page belongs somewhere in the middle of the list, so we need to find the page
        // that comes BEFORE the new page we're inserting.
        let mut parent_page = self.find_insertion_point(new_page)
                                  .expect("Failed to find parent page for new page.");

        // Make sure that the new page is not already in the list.
        assert!(parent_page.address != new_page.address,
                "Trying to insert a duplicate page at 0x{:x} into the free page list.",
                new_page.address);

        // Get the page that will be after the new page we're inserting.
        let original_next_page = parent_page.next_page;

        // Wire up the new page's pointers.
        new_page.prev_page = Some(parent_page);
        new_page.next_page = original_next_page;

        // Make sure that the parent now points to the new page.
        parent_page.next_page = Some(new_page);

        // If there was a page after the parent, it now needs to point back at this new page.
        // Otherwise our new page is the new last page in the list.
        if let Some(mut next_page_ptr) = original_next_page
        {
            next_page_ptr.prev_page = Some(new_page);
        }
        else
        {
            self.last_page = Some(new_page);
        }
    }

    /// Insert a range of free pages into the free page list. This will insert the pages in the
    /// correct position in the list based on their addresses.
    ///
    /// It is a fatal error if the list of pages are not contiguous and in order.
    pub fn insert_page_list(&mut self,
                            mut first_page: FreeMemoryPagePtr,
                            mut last_page: FreeMemoryPagePtr)
    {
        // Validate the incoming list of pages.
        assert!(Self::pages_are_contiguous(first_page, last_page),
                "Pages are not contiguous or in order. First page at 0x{:x}, last page at 0x{:x}.",
                first_page.address,
                last_page.address);

        // If the list is empty, the job is pretty easy. The new list is the whole list.
        if self.is_empty()
        {
            // Just set the first and last page pointers to the new pages.
            self.first_page = Some(first_page);
            self.last_page = Some(last_page);

            return;
        }

        let mut self_first_page = self.first_page.unwrap();

        if self_first_page.address > first_page.address
        {
            assert!(self_first_page.address >= last_page.address,
                    "Trying to insert a duplicate page in a page list at 0x{:x}.",
                    last_page.address);

            // Insert the new list at the beginning of the existing list.
            self.first_page = Some(first_page);

            last_page.next_page = Some(self_first_page);
            self_first_page.prev_page = Some(last_page);

            assert!(self_first_page.prev_page.is_none(),
                    "First page in the list should not have a previous page, but it does.");

            return;
        }

        // Are we inserting the new list at the end of the existing list?
        assert!(self.last_page.is_some(),
                "Free page list is not empty, but last page is None.");

        let mut self_last_page = self.last_page.unwrap();

        if self_last_page.address < first_page.address
        {
            assert!(self_last_page.address <= last_page.address,
                    "Trying to insert a duplicate page in a page list at 0x{:x}.",
                    last_page.address);

            // Insert the new list at the end of the existing list.
            self_last_page.next_page = Some(first_page);
            first_page.prev_page = Some(self_last_page);

            self.last_page = Some(last_page);

            assert!(last_page.next_page.is_none(),
                    "Last page in the list should not have a next page, but it does.");

            return;
        }

        // Find the proper place to insert the list of pages.
        let parent_page = self.find_insertion_point(first_page);

        assert!(parent_page.is_some(), "Failed to find parent page for new page list.");

        let mut parent_page = parent_page.unwrap();

        assert!(parent_page.address != first_page.address,
                "Trying to insert a duplicate page at 0x{:x} into the free page list.",
                first_page.address);

        assert!(parent_page.address < first_page.address,
                "Trying to insert a page at 0x{:x} before parent page at 0x{:x}.",
                first_page.address,
                parent_page.address);

        let original_next_page = parent_page.next_page;

        assert!(original_next_page.is_some(),
                "Parent page should have a next page, but it does not.");

        let mut original_next_page = original_next_page.unwrap();

        parent_page.next_page = Some(first_page);

        first_page.prev_page = Some(parent_page);
        last_page.next_page = Some(original_next_page);

        original_next_page.prev_page = Some(last_page);
    }

    /// Remove a page from the free page list.
    ///
    /// Will return None if the list is empty.
    pub fn remove_page(&mut self) -> Option<FreeMemoryPagePtr>
    {
        // Do we have any pages in the list?
        if self.is_empty()
        {
            return None;
        }

        // Simply pop the first page from the lest and make the next page, (if any) the new top of
        // the list.
        let page_ptr = self.first_page.unwrap();

        self.first_page = page_ptr.next_page;
        Some(page_ptr)
    }

    /// Remove a number of contiguous pages from the free page list. It is guaranteed that the pages
    /// will be contiguous and in order.
    ///
    /// Will return None if the list is empty or there are not enough contiguous pages to satisfy
    /// the request.
    pub fn remove_page_list(&mut self, count: usize) -> Option<FreeMemoryPagePtr>
    {
        // Check if the request makes sense.
        assert!(count > 0, "Can not remove zero pages from the free page list.");

        // Are there any pages in the list?
        if self.is_empty()
        {
            return None;
        }

        // If we're just removing one page then we can just use the remove_page method and skip the
        // extra complexity.
        if count == 1
        {
            return self.remove_page();
        }

        // Start at the beginning of the list and iterate through the pages until we find a set of
        // contiguous pages.
        let mut current_page = self.first_page;

        // Fire through the list and attempt to find the requested number of contiguous pages.
        while let Some(mut current_page_ptr) = current_page
        {
            // Try to find the contiguous pages starting at the current page. If successful we
            // will get a valid last page pointer back.
            if let Some(mut last_page_ptr)
                = Self::find_contiguous_pages(current_page_ptr, count)
            {
                // We found a valid set of pages, so now we need to remove them from the list.
                // Get the pages before and after the set of the pages we found. (If any both
                // prev_page and next_page can be None.)
                let prev_page = current_page_ptr.prev_page;
                let next_page = last_page_ptr.next_page;

                // If we have a previous page, then we need to update its next pointer to point
                // to the next page after the ones we're removing.
                if let Some(mut prev_page_ptr) = prev_page
                {
                    prev_page_ptr.next_page = next_page;
                }
                else
                {
                    // There was no previous page, so the new first page of the list should be
                    // set to the first page after the removal.
                    self.first_page = next_page;
                }

                // If we have a next page after the remove list then we need to update its prev
                // pointer to point to the previous page before the ones we're removing.
                if let Some(mut next_page_ptr) = next_page
                {
                    next_page_ptr.prev_page = prev_page;
                }
                else
                {
                    // We're removing from the end of the list, so we need to update the last
                    // page pointer to the previous page before the ones we're removing.
                    self.last_page = prev_page;
                }

                // Make sure that the pages we found are properly removed from the list.
                current_page_ptr.prev_page = None;
                last_page_ptr.next_page = None;

                // Return the first page in the set of pages we found.
                return Some(current_page_ptr);
            }

            // We didn't find our set of pages, so move on and try again.
            current_page = current_page_ptr.next_page;
        }

        // We didn't find any contiguous pages, so return None.
        None
    }

    /// Check if the free page list is empty. This will return true if there are no pages in the
    /// list, and false if there are pages in the list.
    pub fn is_empty(&self) -> bool
    {
        let empty = self.first_page.is_none();

        // Some safety checks to ensure that the free page list is in a consistent state.

        assert!(if empty { self.last_page.is_none() } else { self.last_page.is_some() },
                "Inconsistent state of free page list. First page, {}, last page, {}.",
                self.first_page.is_some(),
                self.last_page.is_some());

        empty
    }

    /// Check if the next set of pages starting at the given page are contiguous in memory. If they
    /// are contiguous then return the last page in the set, otherwise return None.
    fn find_contiguous_pages(start_page_ptr: FreeMemoryPagePtr,
                             count: usize) -> Option<FreeMemoryPagePtr>
    {
        // If no pages are requested then we can't find any pages.
        if count == 0
        {
            return None;
        }

        // If we're just looking for a single page then any page is automatically the right one.
        if count == 1
        {
            return Some(start_page_ptr);
        }

        // Iterate though the pages and check to see if they are contiguous. It's in an unsafe
        // section because we're doing a lot of pointer manipulation here.
        let mut current_page = start_page_ptr;
        let mut pages_found = 1;

        while pages_found < count
        {
            if let Some(next_page_ptr) = current_page.next_page
            {
                let current_address = current_page.address;
                let next_address = next_page_ptr.address;

                if current_address + PAGE_SIZE == next_address
                {
                    current_page = next_page_ptr;
                    pages_found += 1;
                }
                else
                {
                    // The next page is not contiguous so the search is over.
                    return None;
                }
            }
            else
            {
                // We reached the end of the list, so we can't find any more pages.
                return None;
            }
        }

        // If we got here then we found the requested number of contiguous pages, return the
        // last page in the set.
        Some(current_page)
    }

    /// Check the list of pages to see if they are contiguous and in order.
    fn pages_are_contiguous(first_page: FreeMemoryPagePtr,
                            last_page: FreeMemoryPagePtr) -> bool
    {
        unsafe
        {
            let mut current_page = first_page;

            while current_page.address != last_page.address
            {
                // Check if the next page is contiguous.
                if let Some(next_page) = current_page.next_page
                {
                    // If the next page is not contiguous, then we are done.
                    if next_page.address != current_page.address + PAGE_SIZE
                    {
                        return false;
                    }

                    current_page = next_page;
                }
                else
                {
                    break;
                }
            }

            // Make sure that we found the last page in our iteration. If not, then there is
            // something weird going on.
            assert!(current_page.address == last_page.address,
                    "Last page found address does not match the expected last page address. \
                    Expected 0x{:x}, found 0x{:x}.",
                    last_page.address,
                    current_page.address);
        }

        true
    }

    /// Does a new page logically belong at the beginning of the free page list? This will return
    /// true if the page belongs at the beginning of the list.
    fn is_page_at_beginning(&self, page: FreeMemoryPagePtr) -> bool
    {
        if let Some(first_page) = self.first_page
        {
            // Make sure that this isn't a duplicate page.
            assert!(first_page.address != page.address,
                    "Trying to insert a duplicate page at 0x{:x} before first page at 0x{:x}.",
                    page.address,
                    first_page.address);

            return first_page.address > page.address;
        }

        false
    }

    /// Does a new page logically belong at the end of the free page list? This will return true if
    /// the page belongs at the end of the list.
    fn is_page_at_end(&self, page: FreeMemoryPagePtr) -> bool
    {
        if let Some(last_page) = self.last_page
        {
            // Make sure that this isn't a duplicate page.
            assert!(last_page.address != page.address,
                    "Trying to insert a duplicate page at 0x{:x} after last page at 0x{:x}.",
                    page.address,
                    last_page.address);

            return last_page.address < page.address;
        }

        false
    }

    /// Iterate through the pages until we find the proper place to insert the new given page.
    fn find_insertion_point(&self, new_page: FreeMemoryPagePtr) -> Option<FreeMemoryPagePtr>
    {
        // If the list is empty then there is no parent page.
        if self.is_empty()
        {
            return None;
        }

        // We assume that this function is only called for pages that are not at the beginning of
        // the list.
        let mut current_page = self.first_page;
        let new_page_address = new_page.address;

        while let Some(current_page_ptr) = current_page
        {
            // Check the next page, if there is no next page or if the next page's address is
            // greater than our new page's address then the current page is the correct parent
            // page for our insertion.
            if let Some(next_page_ptr) = current_page_ptr.next_page
            {
                if next_page_ptr.address > new_page_address
                {
                    return Some(current_page_ptr);
                }
            }
            else
            {
                // There is no next page, so the current page has to be the insertion point.
                return Some(current_page_ptr);
            }

            // Move to the next page in the list.
            current_page = current_page_ptr.next_page;
        }

        // This code shouldn't be reached.
        unreachable!();
    }
}



/// Keep an internal global reference to our free page list. That we are using a struct for this is
/// an internal implementation detail, the API is what matters to the MMU handling.
///
/// Again, it is up to the calling code to ensure all accesses to this API are thread safe and that
/// the free page list is not modified while it is being read.
static mut FREE_PAGE_LIST: FreePageList = FreePageList::new();



/// Initialize the free page list to include all the free pages not used by either the kernel and
/// the attached MMIO devices. All found memory devices will be added to the free page list as if
/// they were one device. All gaps in address ranges will be skipped and the calling code will not
/// need to worry about handing out non-existent memory pages.
pub fn init_free_page_list(kernel_memory: &KernelMemoryLayout,
                           system_memory: &SystemMemory)
{
    /// Check if the address is within the kernel memory range, or part of the heap that will be
    /// used by the kernel later.
    fn is_kernel_page(address: usize, kernel_memory: &KernelMemoryLayout) -> bool
    {
        (   address >= kernel_memory.kernel.start
         && address <  kernel_memory.kernel.start + kernel_memory.kernel.size)

        ||

        (   address >= kernel_memory.heap.start
         && address <  kernel_memory.heap.start + kernel_memory.heap.size)
    }

    // Check if the address is within a MMIO device range.
    fn is_mmio_page(address: usize, system_memory: &SystemMemory) -> bool
    {
        for mmio_region in &system_memory.mmio_regions
        {
            if let Some(mmio_region) = mmio_region
            {
                let result =    address >= mmio_region.base_address
                             && address < (mmio_region.base_address + mmio_region.range);

                if result
                {
                    return true;
                }
            }
        }

        false
    }

    // Ok, lets iterate all the memory devices we've detected in the system and add their memory to
    // our free page list.
    for memory_device in &system_memory.memory_devices
    {
        // Not every entry in the list will be populated, so we need to check if we have a valid
        // memory device.
        if let Some(memory_device) = memory_device
        {
            // Get the starting and ending addresses of the memory device.
            let start_address = memory_device.base_address;
            let end_address = memory_device.base_address + memory_device.range;

            // Make sure that the device's page layout makes sense.
            assert!(start_address % PAGE_SIZE == 0,
                    "Memory device start address must be aligned to page boundary, got 0x{:x}. \
                    Page size configured as {} bytes.",
                    start_address,
                    PAGE_SIZE);

            assert!(end_address % PAGE_SIZE == 0,
                    "Memory device end address must be aligned to page boundary, got 0x{:x}. \
                    Page size configured as {} bytes.",
                    end_address,
                    PAGE_SIZE);

            assert!(memory_device.range != 0,
                    "Memory device range must be greater than zero, got 0x{:x}.",
                    memory_device.range);

            // Iterate over the memory device's pages and add them to the free page list. Unless
            // that page belongs to the kernel or is used by a MMIO device.
            for page_address in (start_address..end_address).step_by(PAGE_SIZE)
            {
                if    !is_kernel_page(page_address, kernel_memory)
                   && !is_mmio_page(page_address, system_memory)
                {
                    // Add the page to the end of the free page list.
                    let page_ptr = FreeMemoryPage::new(page_address, None, None);
                    let free_page_list = &raw mut FREE_PAGE_LIST;

                    unsafe
                    {
                        (*free_page_list).add_free_page_to_end(page_ptr);
                    }
                }
            }
        }
    }
}



/// Add a free page to the free page list.
pub fn add_free_page(page_address: usize)
{
    assert!(page_address % PAGE_SIZE == 0,
            "Page address must be aligned to page boundary, got 0x{:x}.",
            page_address);

    unsafe
    {
        let page_ptr = FreeMemoryPage::new(page_address, None, None);
        let free_page_list = &raw mut FREE_PAGE_LIST;

        (*free_page_list).insert_page(page_ptr);
    }
}



/// Add a number of contiguous free pages to the free page list.
pub fn add_n_free_pages(address: usize, count: usize)
{
    // Validate the incoming address and count.
    assert!(address % PAGE_SIZE == 0,
            "Address must be aligned to page boundary, got 0x{:x}.",
            address);

    assert!(count > 0, "Count must be greater than zero, got {}.", count);

    unsafe
    {
        // Create the head of the new list.
        let free_page_head = FreeMemoryPage::new(address, None, None);

        let mut current_page_ptr = free_page_head;

        // Iterate over the number of pages and create the linked list of free pages.
        for index in 1..count
        {
            // Calculate the address of the page based on the index and the base address.
            let page_address = address + (index * PAGE_SIZE);
            let mut new_page_ptr = FreeMemoryPage::new(page_address, None, None);

            // Link the new page into the list.
            current_page_ptr.next_page = Some(new_page_ptr);
            new_page_ptr.prev_page = Some(current_page_ptr);

            current_page_ptr = new_page_ptr;
        }

        // Now we have our list of free pages, we can add it to the official free page list.
        let free_page_list = &raw mut FREE_PAGE_LIST;

        (*free_page_list).insert_page_list(free_page_head, current_page_ptr);
    }
}



/// Attempt to pull a free page from the free page list.
///
/// This will return None if there are no free pages available in the list.
///
/// This function makes no guarantees about the page's address other than it is a valid page as
/// given to the list from the memory subsystem.
pub fn remove_free_page() -> Option<usize>
{
    // Get the free page list and attempt to remove a page from it.
    let free_page_list = &raw mut FREE_PAGE_LIST;
    let page_ptr = unsafe { (*free_page_list).remove_page() };

    // Check to see if we got a page pointer back.
    if let Some(mut page_ptr) = page_ptr
    {
        unsafe
        {
            // We did, so extract the address from the page pointer and clear the page's internal
            // bookkeeping so that we don't leak any internal data. Then return the address of the page.
            let address = page_ptr.address;

            page_ptr.clear();
            Some(address)
        }
    }
    else
    {
        // There wasn't a page available, so return None.
        None
    }
}



/// Attempt to pull a number of contiguous free pages from the free page list.
///
/// This will return None if there are not enough contiguous free pages available in the list.
///
/// This function makes no guarantees about the pages' addresses other than they are valid pages as
/// given to the list from the memory subsystem.
pub fn remove_n_free_pages(count: usize) -> Option<usize>
{
    // Ok, get a reference to the free list and try to extract the requested number of pages.
    let free_page_list = &raw mut FREE_PAGE_LIST;
    let first_page_ptr = unsafe { (*free_page_list).remove_page_list(count) };

    // Did we get a list of pages back?
    if first_page_ptr.is_some()
    {
        // Get the address of the first page in the list.
        let address = unsafe { (*first_page_ptr.unwrap()).address };

        // Iterate through the pages and clear their internal bookkeeping.
        let mut current_page_ptr = first_page_ptr;

        while current_page_ptr.is_some()
        {
            // Get the current page pointer.
            let mut page_ptr = current_page_ptr.unwrap();

            // Extract the next page from the list then clear out the current page's bookkeeping.
            current_page_ptr = page_ptr.next_page;
            page_ptr.clear();
        }

        // Return the address of the first page in the list.
        Some(address)
    }
    else
    {
        // There were either no pages available or not enough contiguous pages, so return None.
        None
    }
}
