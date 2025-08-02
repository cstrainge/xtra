
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

use core::{ fmt::{ self, Display, Formatter },
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
                write!(f, "Virtual address {} is outside of the valid range [{}, {}]",
                       address,
                       min,
                       max),

            AddressError::BadPhysicalAddress { address, max } =>
                write!(f, "Physical address {} is outside of the valid range [0, {}]",
                       address,
                       max),

            AddressError::BadPageAlignment { address, alignment } =>
                write!(f, "Address {} is not properly aligned to a page boundary of {}",
                       address,
                       alignment)
        }
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



/// TODO: Make this a kernel configuration option so that we can change the virtual base offset at
///       compile time.
///
/// The base virtual address for the kernel's physical free page management. All free pages in the
/// system will be mapped into this virtual address space so that the kernel can still access the
/// physical pages directly as needed. For example mapping a page into an address space.
///
/// TODO: Right now we are only allowing for 4GB of actual RAM, we need to make this computed at
///       runtime based on the system's memory layout.
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



/// A struct that maintains addresses for our pages of physical memory. These addresses can be
/// either within the virtual address space or in the physical address space depending on the mode
/// kernel is in.
///
/// This struct helps manage the distinction between physical and virtual addresses. Because a valid
/// pointer in one mode would be an invalid pointer in the other mode.
#[repr(transparent)]
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct VirtualPageAddress(usize);



impl VirtualPageAddress
{
    /// Create a new virtual address from a raw typed pointer. Internally this will make sure that
    /// the address is in the virtual address space.
    ///
    /// Receiving a NULL pointer or a non page aligned pointer will result in an error.
    pub fn from_ptr<T>(address: *const T) -> Result<Self>
    {
        // Convert to our internal format.
        let address = address as usize;

        // Make sure that the given address is aligned to the page boundary.
        if address % PAGE_SIZE != 0
        {
            return Err(AddressError::BadPageAlignment
                {
                    address,
                    alignment: PAGE_SIZE
                });
        }

        // Make sure that the address is not a null pointer.
        if address == 0
        {
            return Err(AddressError::Null);
        }

        // Check to see if there is conversion required based on the kernel's mode.
        if is_kernel_in_virtual_mode()
        {
            // The kernel is in virtual mode so we can just use the address as is.
            Self::from_virtual(address as usize)
        }
        else
        {
            // The kernel is in physical mode, so we need to convert the address to a virtual one.
            Self::from_physical(address as usize)
        }
    }

    /// Create a new virtual address structure from an existing physical address value.
    ///
    /// This will fail if the address is outside of the physical address space or if the value is
    /// zero.
    pub fn from_physical(physical_address: usize) -> Result<Self>
    {
        let highest = highest_physical_address();

        // Make sure that the page address is aligned to the page size.
        if physical_address % PAGE_SIZE != 0
        {
            return Err(AddressError::BadPageAlignment
                {
                    address: physical_address,
                    alignment: PAGE_SIZE
                });
        }

        match physical_address
        {
            0 =>
                {
                    Err(AddressError::Null)
                },

            _ if physical_address >= highest =>
                {
                    Err(AddressError::BadPhysicalAddress
                        {
                            address: physical_address,
                            max: highest
                        })
                },

            _ =>
                {
                    Ok(Self(physical_address + virtual_base_offset()))
                }
        }
    }

    /// Create a new virtual address structure from an existing virtual address value.
    ///
    /// We make sure that the virtual address is in the correct range.
    pub fn from_virtual(virtual_address: usize) -> Result<Self>
    {
        let virtual_base = virtual_base_offset();

        if virtual_address % PAGE_SIZE != 0
        {
            return Err(AddressError::BadPageAlignment
                {
                    address: virtual_address,
                    alignment: PAGE_SIZE
                });
        }

        match virtual_address
        {
            0 =>
                {
                    Err(AddressError::Null)
                },

            _ if    virtual_address < virtual_base
                 || virtual_address > HIGHEST_VIRTUAL_ADDRESS =>
                {
                    Err(AddressError::BadVirtualAddress
                        {
                            address: virtual_address,
                            min: virtual_base,
                            max: HIGHEST_VIRTUAL_ADDRESS
                        })
                },

            _ =>
                {
                    Ok(Self(virtual_address))
                }
        }
    }

    /// Explicitly convert this virtual address to a physical address.
    pub fn to_physical(&self) -> usize
    {
        // Translate the virtual address back to a physical address by subtracting the
        // virtual base offset.
        self.0 - virtual_base_offset()
    }

    /// Explicitly get the virtual address from this virtual address structure.
    pub fn to_virtual(&self) -> usize
    {
        // No translation needed, just return the address.
        self.0
    }

    /// Get the raw address of this virtual address, depending on the mode the kernel is in.
    ///
    /// If the kernel is in virtual mode then this will return the virtual address, otherwise it
    /// will return the physical address.
    pub fn to_usize(&self) -> usize
    {
        if is_kernel_in_virtual_mode()
        {
            // The kernel is in virtual mode so return the address as a virtual address.
            self.to_virtual()
        }
        else
        {
            // Convert from the virtual address space to the physical address space.
            self.to_physical()
        }
    }

    /// Convert this virtual address to a raw pointer depending on the mode the kernel is in.
    pub fn to_ptr<T>(&self) -> *const T
    {
        self.to_usize() as *const T
    }

    /// Convert this virtual address to a mutable raw pointer depending on the mode the kernel is
    /// in.
    pub fn to_mut_ptr<T>(&self) -> *mut T
    {
        self.to_usize() as *mut T
    }
}



impl Display for VirtualPageAddress
{
    /// Format the virtual page address for display to the user when needed. This is only safe to
    /// call once the memory manager has been initialized.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        write!(f, "VPA({:#x}/{:#x})", self.0, self.to_physical())
    }
}
