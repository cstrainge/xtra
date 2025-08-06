
// When dealing with pages of physical memory the kernel has two modes. It can either be in physical
// address mode. (The mode it is in at startup,) or in virtual address mode. (After the memory
// subsystem has been initialized.)
//
// In physical address mode all addresses to the page are exactly as they are as mapped by the
// actual hardware.
//
// However, when switching to its virtual addressing mode all of the pages of physical memory are
// remapped to an upper address space in RAM. This allows the kernel to access these pages without
// clobbering the address space of itself and user processes.
//
// These special pages are mapped into all address spaces so that the kernel can access them quickly
// and easily.

use core::{ any::type_name,
            convert::TryFrom,
            fmt::{ self, Debug, Display, Formatter },
            ops::{ Deref, DerefMut },
            ptr::{ from_raw_parts, from_raw_parts_mut, metadata },
            sync::atomic::{ AtomicBool, AtomicUsize, Ordering } };

use crate::{ arch::mmu::HIGHEST_VIRTUAL_ADDRESS,
             memory::mmu::{ get_system_memory_layout, PAGE_SIZE } };



/// Possible errors that can occur when converting from raw values or pointers to the virtual
/// address structure.
pub enum AddressError
{
    /// The given address is a null pointer.
    Null,

    /// The given virtual address is outside of the valid range for the kernel's virtual page
    /// address space.
    BadVirtualAddress { address: usize, min: usize, max: usize },

    /// The given physical address is outside of the valid range for the kernel's physical page
    /// address space. The value is the invalid address.
    BadPhysicalAddress { address: usize, max: usize },

    /// Attempted to create a memory page address that wasn't properly aligned to a page boundary.
    BadPageAlignment { address: usize, alignment: usize }
}



impl Display for AddressError
{
    /// Format the address error for display to the user when needed.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        match self
        {
            AddressError::Null =>
                write!(f, "Address is a null pointer"),

            AddressError::BadVirtualAddress { address, min, max } =>
                write!(f, "Virtual address {} is outside of the valid range [{}, {}].",
                       address,
                       min,
                       max),

            AddressError::BadPhysicalAddress { address, max } =>
                write!(f, "Physical address {} is outside of the valid range [0, {}].",
                       address,
                       max),

            AddressError::BadPageAlignment { address, alignment } =>
                write!(f, "Address {} is not properly aligned to a page boundary of {}.",
                       address,
                       alignment)
        }
    }
}



impl Debug for AddressError
{
    /// Format the address error for debugging purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        Display::fmt(self, f)
    }
}



/// The result type that the address constructors can return.
type Result<T> = core::result::Result<T, AddressError>;



/// Keep track of whether the kernel has switched to it's virtual address space or not.
static KERNEL_IN_VIRTUAL_MODE: AtomicBool = AtomicBool::new(false);



/// Check if the kernel is currently in virtual mode. This means that the kernel is running under
/// a virtual address space and not the raw physical address space.
pub fn is_kernel_in_virtual_mode() -> bool
{
    KERNEL_IN_VIRTUAL_MODE.load(Ordering::Acquire)
}



/// On boot once the kernel's virtual address space has been created we will switch to it and start
/// using it for all memory accesses.
///
/// Certain sub-systems need to know this like the free page manager so that they can properly map
/// virtual addresses to physical addresses and vice versa.
pub fn set_kernel_in_virtual_mode()
{
    KERNEL_IN_VIRTUAL_MODE.store(true, Ordering::Release);
}



/// The base virtual address for the kernel's physical free page management. All free pages in the
/// system will be mapped into this virtual address space so that the kernel can still access the
/// physical pages directly as needed. For example mapping a page into an address space.
///
/// This is computed during the kernel's MMU initialization and is used to map all physical pages
/// into the kernel's virtual address space.
///
/// We want to minimize the amount of address space used but are also constrained by the underlying
/// architecture's maximum addressable space. Many architectures have a limit that's far below what
/// a 64-bit address space could conceptually support.
///
/// For example, the RISC-V 64-bit architecture has a maximum addressable space of 512GB.
static VIRTUAL_BASE_OFFSET: AtomicUsize  = AtomicUsize::new(0);



/// The highest physical address found in the system. This helps compute the virtual base offset
/// for the kernel's physical free page management.
static HIGHEST_PHYSICAL_ADDRESS: AtomicUsize = AtomicUsize::new(0);



/// Align an address up to the nearest multiple of the given alignment.
const fn align_up(address: usize, alignment: usize) -> usize
{
    (address + (alignment - 1)) & !(alignment - 1)
}



/// Align an address down to the nearest multiple of the given alignment.
const fn align_down(address: usize, alignment: usize) -> usize
{
    address & !(alignment - 1)
}



/// Get the virtual base offset for the kernel's physical free page management.
#[inline(always)]
fn virtual_base_offset() -> usize
{
    // Load the virtual base offset from the atomic variable.
    let offset = VIRTUAL_BASE_OFFSET.load(Ordering::Acquire);

    // Ensure that the virtual base offset has been initialized.
    debug_assert!(offset != 0, "Virtual base offset must be initialized before use.");

    offset
}



/// Get the highest usable physical address in RAM. During startup we compute the highest mapped RAM
/// device's address.
#[inline(always)]
fn highest_physical_address() -> usize
{
    let address = HIGHEST_PHYSICAL_ADDRESS.load(Ordering::Acquire);

    // Ensure that the address has been initialized during startup.
    debug_assert!(address != 0, "Highest physical address must be initialized before use.");

    address
}



/// Initialize the virtual base offset for the kernel's physical free page management once we've
/// switched to the virtual address space.  All physical pages will be remapped to their virtual
/// addresses based on this offset.
pub fn init_virtual_base_offset()
{
    // Make sure that we aren't doing a double initialization.
    debug_assert!(VIRTUAL_BASE_OFFSET.load(Ordering::Relaxed) == 0,
                  "init_virtual_base_offset() called twice");

    // Get the system memory layout to find the highest used address in the system.
    let memory_layout = get_system_memory_layout();

    // Start off with no RAM allocated.
    let mut highest_address = 0;

    // TODO: We could minimize the amount of address space used by also figuring out the lowest
    //       used address and narrow the window down to the used addresses.

    // Iterate over the found memory devices and find the highest used address in the system.
    for device in memory_layout.memory_devices
    {
        if let Some(device) = device
        {
            highest_address = highest_address.max(device.base_address + device.range);
        }
    }

    // Align up the highest address to make sure that the last full page fits.
    highest_address = align_up(highest_address, PAGE_SIZE);

    // Ok, we have the highest address in the system, now we can setup a virtual base offset that
    // can accommodate the entire physical address space.
    //
    // While doing so make sure that the lowest address will end up being page aligned.
    let virtual_base_offset = align_down(HIGHEST_VIRTUAL_ADDRESS - highest_address, PAGE_SIZE);

    // Keep our computed values for later use.
    HIGHEST_PHYSICAL_ADDRESS.store(highest_address, Ordering::Release);
    VIRTUAL_BASE_OFFSET.store(virtual_base_offset, Ordering::Release);
}



pub fn virtualize_address(address: usize) -> usize
{
    // Otherwise we need to convert the physical address to a virtual one.
    virtual_base_offset() + address
}



/// A struct that maintains addresses for our pages of physical memory. These addresses can be
/// either within the virtual address space or in the physical address space depending on the mode
/// kernel is in.
///
/// This struct helps manage the distinction between physical and virtual addresses. Because a valid
/// pointer in one mode would be an invalid pointer in the other mode.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct VirtualPagePtr<T: ?Sized>
{
    /// The underlying raw pointer to the virtual page data.
    raw_ptr: *mut T
}



impl<T> VirtualPagePtr<T>
{
    /// Create a new VirtualPagePtr from a raw pointer. In either the virtual or physical address
    /// space depending on the kernel's mode.
    pub fn new(raw_ptr: *mut T) -> Result<Self>
    {
        Self::new_from_address(raw_ptr as usize)
    }

    /// Create a new VirtualPagePtr from a physical or virtual address depending on the kernel mode.
    pub fn new_from_address(address: usize) -> Result<Self>
    {
        // Check to see if there is conversion required based on the kernel's mode.
        if is_kernel_in_virtual_mode()
        {
            // The kernel is in virtual mode so we can just use the address as is.
            Self::from_virtual(address)
        }
        else
        {
            // The kernel is in physical mode, so we need to convert the address to a virtual one.
            Self::from_physical(address)
        }
    }

    /// Create a new VirtualPagePtr from an existing virtual address. That is a page address in the
    /// virtual address space of the kernel.
    ///
    /// If the address isn't valid then this will return an error instead.
    pub fn from_virtual(address: usize) -> Result<Self>
    {

        // Check if the address is properly aligned to a page boundary.
        if address % PAGE_SIZE != 0
        {
            return Err(AddressError::BadPageAlignment { address, alignment: PAGE_SIZE });
        }

        // Check if the address makes sense and if it does then create a new VirtualPagePtr.
        let virtual_base = virtual_base_offset();

        match address
        {
            0 =>
                {
                    // This is a null pointer, so we can't create a valid VirtualPagePtr.
                    Err(AddressError::Null)
                },

            _ if !Self::is_in_virtual_address_space(address) =>
                {
                    // This address doesn't make sense in the virtual address space.
                    Err(AddressError::BadVirtualAddress
                        {
                            address,
                            min: virtual_base,
                            max: HIGHEST_VIRTUAL_ADDRESS
                        })
                },

            _ =>
                {
                    // The address is valid, so we can create a new VirtualPagePtr.
                    Ok(Self { raw_ptr: address as *mut T })
                }
        }
    }

    /// Create a new VirtualPagePtr from an existing physical address, that is an address in the
    /// physical address space of the system.
    ///
    /// If the address isn't valid then this will return an error instead.
    pub fn from_physical(address: usize) -> Result<Self>
    {
        // Is the new address page aligned?
        if address % PAGE_SIZE != 0
        {
            return Err(AddressError::BadPageAlignment { address, alignment: PAGE_SIZE });
        }

        // Check if the address is within the valid range of physical addresses.
        let highest_address = highest_physical_address();

        match address
        {
            0 =>
                {
                    // This is a null pointer, so we can't create a valid VirtualPagePtr.
                    Err(AddressError::Null)
                },

            _ if !Self::is_in_physical_address_space(address) =>
                {
                    // This address doesn't make sense in the physical address space.
                    Err(AddressError::BadPhysicalAddress { address, max: highest_address })
                },

            _ =>
                {
                    // The address is valid, so we can create a new VirtualPagePtr. So shift the
                    // physical address into the virtual address space.
                    let virtual_address = virtualize_address(address);

                    Ok(Self { raw_ptr: virtual_address as *mut T })
                }
        }
    }

    /// Check if the given address is within the valid range of virtual addresses.
    pub fn is_in_virtual_address_space(address: usize) -> bool
    {
        // Check if the address is within the valid range of virtual addresses.
           address >= virtual_base_offset()
        && address <= HIGHEST_VIRTUAL_ADDRESS
    }

    /// Check if the given address is within the valid range of physical addresses.
    pub fn is_in_physical_address_space(address: usize) -> bool
    {
        // Check if the address is within the valid range of physical addresses.
           address < highest_physical_address()
        && address > 0
    }

    pub fn as_physical_address(&self) -> usize
    {
        self.raw_ptr as usize - virtual_base_offset()
    }
}



impl<T: ?Sized> VirtualPagePtr<T>
{
    /// Get the raw pointer as an address, either virtual or physical depending on the kernel's
    /// current address mode.
    pub fn as_usize(&self) -> usize
    {
        /// Convert a virtual address to a physical address. Should only be called during boot time
        /// when the kernel is still in physical address mode.
        ///
        /// So we hint to the compiler that this function should only be run rarely.
        #[cold]
        fn devirtualize(address: usize) -> usize
        {
            // Devirtualize the address by subtracting the virtual base offset.
            address - virtual_base_offset()
        }

        // Convert the raw pointer to an address. Then check to see if the conversion is needed or
        // not.
        let raw_size = self.raw_ptr as *const u8 as usize;
        let mut raw_size = if is_kernel_in_virtual_mode()
            {
                raw_size
            }
            else
            {
                devirtualize(raw_size)
            };

        // Give the caller what they want.
        raw_size
    }

    pub fn as_ptr(&self) -> *const T
    {
        let address = self.as_usize();

        from_raw_parts(address as *const (), metadata(self.raw_ptr))
    }

    pub fn as_mut_ptr(&mut self) -> *mut T
    {
        let address = self.as_usize();

        from_raw_parts_mut(address as *mut (), metadata(self.raw_ptr))
    }
}



impl<T> Deref for VirtualPagePtr<T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { &*self.as_ptr() }
    }
}



impl<T> DerefMut for VirtualPagePtr<T>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        unsafe { &mut *self.as_mut_ptr() }
    }
}



impl<T> TryFrom<*mut T> for VirtualPagePtr<T>
{
    type Error = AddressError;

    fn try_from(raw_ptr: *mut T) -> Result<Self>
    {
        Self::new(raw_ptr)
    }
}



impl<T> TryFrom<usize> for VirtualPagePtr<T>
{
    type Error = AddressError;

    fn try_from(address: usize) -> Result<Self>
    {
        Self::new_from_address(address)
    }
}



impl<T: ?Sized> From<&VirtualPagePtr<T>> for usize
{
    fn from(ptr: &VirtualPagePtr<T>) -> usize
    {
        ptr.as_usize()
    }
}



impl<T: ?Sized> From<&mut VirtualPagePtr<T>> for usize
{
    fn from(ptr: &mut VirtualPagePtr<T>) -> usize
    {
        ptr.as_usize()
    }
}



impl<T> Display for VirtualPagePtr<T>
{
    /// Format the virtual page address for display to the user when needed. This is only safe to
    /// call once the memory manager has been initialized.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        let address = self.as_usize();
        write!(f, "VPA<{}>({:#x}/{:#x})",
               type_name::<T>(),
               address,
               address - virtual_base_offset())
    }
}
