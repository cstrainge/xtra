
// The heap allocator for the kernel. This code provides the implementation of the global allocator
// for the kernel, which enabled things like Box, Vec, String and other dynamic memory structures.

use core::{ alloc::{ GlobalAlloc, Layout },
            arch::asm,
            ptr::null_mut,
            sync::atomic::{ AtomicBool, AtomicUsize, Ordering } };

use crate::memory::kernel::{ KernelMemoryLayout, SectionLayout };



/// The one and only global allocator for the kernel. This is the allocator that will be used for
/// all dynamic memory allocations in the kernel.
#[global_allocator]
static HEAP_ALLOCATOR: HeapAllocator = HeapAllocator::new();



/// Temporary allocation error handler for the kernel. This will be called if the heap allocator
/// fails to allocate memory.
#[cfg(feature = "nightly")]
#[alloc_error_handler]
fn alloc_error(layout: Layout) -> !
{
    panic!("Kernel heap allocation failed, size: {}, align: {}", layout.size(), layout.align());
}



/// The heap allocator structure itself.
struct HeapAllocator
{
    /// The beginning of the heap in RAM. Atomic for multi-core safe access.
    heap_start: AtomicUsize,

    /// The end of the heap in RAM. Atomic for multi-core safe access.
    heap_end: AtomicUsize,

    /// The next address to try to allocate at in the heap. Atomic for multi-core safe access.
    next_address: AtomicUsize,

    /// Whether the heap allocator has been initialized and is ready for use. Atomic for multi-core
    /// safe access.
    initialized: AtomicBool
}



impl HeapAllocator
{
    /// Default creator for the heap allocator. We start off in an uninitialized state and will wait
    /// for the kernel to be ready enough to properly initialize the heap.
    const fn new() -> HeapAllocator
    {
        HeapAllocator
            {
                heap_start: AtomicUsize::new(0),
                heap_end: AtomicUsize::new(0),
                next_address: AtomicUsize::new(0),
                initialized: AtomicBool::new(false)
            }
    }

    /// Now that the Kernel has the information it needs about the memory layout of the heap, we can
    /// initialize the heap allocator and make it ready for use.
    fn initialize(&self, layout: &SectionLayout) -> Result<(), &'static str>
    {
        // Can't initialize the heap allocator twice, that would be a bug in the kernel and we
        // should be reported fast and early.
        if self.initialized()
        {
            return Err("Heap allocator already initialized");
        }

        // Does the heap even make sense?
        if layout.start >= layout.end || layout.size == 0
        {
            return Err("Invalid heap memory layout");
        }

        // Because we're just a simple bump allocator there isn't much to do here.
        self.heap_start.store(layout.start, Ordering::SeqCst);
        self.heap_end.store(layout.end, Ordering::SeqCst);
        self.next_address.store(layout.start, Ordering::SeqCst);
        self.initialized.store(true, Ordering::SeqCst);

        Ok(())
    }

    /// Is the heap allocator initialized?
    fn initialized(&self) -> bool
    {
        self.initialized.load(Ordering::SeqCst)
    }

    /// Where in RAM does the heap start?
    fn heap_start(&self) -> usize
    {
        self.heap_start.load(Ordering::SeqCst)
    }

    /// Where in RAM does the heap end?
    fn heap_end(&self) -> usize
    {
        self.heap_end.load(Ordering::SeqCst)
    }

    /// Where is the next allocation going to be made in the heap?
    fn next_address(&self) -> usize
    {
        self.next_address.load(Ordering::SeqCst)
    }

    /// Allocate RAM by bumping the next address up past the location required for the allocation.
    fn set_next_address(&self, address: usize)
    {
        self.next_address.store(address, Ordering::SeqCst);
    }

    /// Aligns the given address up to the next multiple of the given alignment.
    fn align_up(address: usize, align: usize) -> usize
    {
        // Is this even a valid alignment?
        assert!(align.is_power_of_two(), "Alignment must be a power of two");

        // Safely compute the aligned address while catching integer overflow. If the heap makes
        // sense within a real memory mep, this is highly unlikely to ever actually happen, but we
        // should still be careful to avoid it if it does.
        match address.checked_add(align - 1)
        {
            Some(value) => value & !(align - 1),
            None        => panic!("Address overflow when trying to align up")
        }
    }
}



unsafe impl GlobalAlloc for HeapAllocator
{
    /// Allocates memory from the heap by bumping the next address up past the location required for
    /// the allocation. This is a very simple and fast allocation strategy, but it does not support
    /// deallocation or reuse of memory. This is a temporary allocator that will be replaced with a
    /// more sophisticated allocator in the future.
    unsafe fn alloc(&self, layout: Layout) -> *mut u8
    {
        // Sanity check that the heap allocator is actually initialized before we try to allocate
        // memory. If the Kernel is trying to allocate memory before the heap has been initialized
        // then something is very wrong and we should panic rather than returning a null pointer and
        // potentially hiding the logic error.
        assert!(self.initialized(), "Heap allocator not initialized");

        // Grab our basic information about the allocation request and the heap layout.
        let align = layout.align();
        let size = layout.size();
        let end = self.heap_end();

        // Compute the next address to try to allocate at, and make sure that the allocation will
        // fit within our heap.
        let mut next_address = self.next_address();

        if next_address >= end
        {
            return null_mut();
        }

        // We're avoiding using an expensive lock here by using atomics. So how this works is that
        // we loop until we can successfully allocate the memory we need.
        //
        // If the memory runs out under us then we will return a null pointer.
        loop
        {
            // First off we need to align the allocation to the next logical address. If that pushes
            // us pass the end of the heap then we need to return a null pointer.
            let aligned = Self::align_up(next_address, align);
            let new_next_address = match aligned.checked_add(size)
                {
                    Some(value) => value,
                    None        => return null_mut()
                };

            // Now does the full aligned allocation fit in the heap?
            if new_next_address > end
            {
                return null_mut();
            }

            // The heart of the allocation algorithm. We attempt to bump the next address up to the
            // new next address past the allocation. If another thread bumped the allocation before
            // us then the compare_exchange will fail and we will get the new next address back.
            // Otherwise we won the allocation race, (if any,) and we can return the aligned address
            // as a pointer to the allocated memory.
            match self.next_address.compare_exchange(next_address,
                                                     new_next_address,
                                                     Ordering::AcqRel,
                                                     Ordering::Relaxed)
            {
                // We won the allocation, convert the address to a proper pointer.
                Ok(_)        => return aligned as *mut u8,

                // We lost the allocation, adjust our starting point and try again.
                Err(current) => next_address = current
            }
        }
    }

    /// A bump allocator never actually frees any memory. So, this function doesn't actually do
    /// anything. Later when the allocator is upgraded this function will return the memory back to
    /// the free pool.
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout)
    {
        unsafe
        {
            asm!("nop", options(nomem, nostack, preserves_flags));
        }
    }
}



/// Initializes the heap allocator for the kernel. This will set up the heap memory region and make
/// it available for allocation and deallocation.
pub fn initialize_heap(memory_layout: &KernelMemoryLayout) -> Result<(), &'static str>
{
    HEAP_ALLOCATOR.initialize(&memory_layout.heap)
}
