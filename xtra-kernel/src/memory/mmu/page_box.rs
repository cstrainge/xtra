
// Implementation of a box type that works directly with pages of memory. The contained type must
// implement the `PageBoxable` trait which allows it to be constructed from a page of memory.
//
// The contained type must also fit withing exactly a single page of memory. The exact size of a
// page is configured by the `PAGE_SIZE` constant in the `mmu` module.

use core::{ any::type_name, ops::{ Deref, DerefMut, Drop }, ptr::drop_in_place };

use crate::memory::{ mmu::{ allocate_page, free_page, virtual_page_ptr::VirtualPagePtr },
                     PAGE_SIZE };



/// A trait that allows a type to be boxed in a page of memory. This is used to create a box that
/// works directly with pages of memory. The type is then given a pointer to the page of memory it
/// will occupy allowing the type to properly initialize itself in the page.
pub trait PageBoxable
{
    /// Allow the boxed item to be constructed directly from a page of memory without needing to
    /// allocate it's information on the stack.
    unsafe fn init_in_place(page_address: &mut VirtualPagePtr<Self>);
}



/// A box that works directly with pages of memory. It is a wrapper around a pointer to a type that
/// implements the `PageBoxable` trait. This allows us to allocate a page of memory use it as a box
/// for the type, and then free the page when the box is dropped.
#[repr(transparent)]
pub struct PageBox<T: ?Sized>
{
    pointer: VirtualPagePtr<T>
}



impl<T> PageBox<T>
{
    /// Create a new `PageBox` for the given type. This will allocate a page of memory and return a
    /// `PageBox` that wraps the pointer to the allocated page.
    pub fn new() -> Self
        where T: PageBoxable + Sized
    {
        // Ensure that the type will fit in a page of memory.
        assert!(size_of::<T>() <= PAGE_SIZE,
                "The size of the type {} must fit in a page of memory. \
                The type is {} bytes, but max is {} bytes.",
                type_name::<T>(),
                size_of::<T>(),
                PAGE_SIZE);

        // Attempt to allocate a page of memory for the box.
        let page_address = allocate_page();

        assert!(page_address.is_some(),
                "Failed to allocate a page for the PageBox for type {}.",
                type_name::<T>());

        let page_address = page_address.unwrap();

        // Create a virtual page pointer from the allocated page address.
        let mut pointer = VirtualPagePtr::new_from_address(page_address)
            .expect("Failed to create a virtual page pointer from the allocated page address.");

        unsafe
        {
            // Allow the type to initialize itself in the allocated page of memory.
            T::init_in_place(&mut pointer);
        }

        Self { pointer }
    }

    /// Create a new 'PageBox' from an existing physical page of memory. This will take ownership of
    /// the page and will free it back to the kernel's memory manager when the box is dropped.
    pub fn from_physical_address(page_address: usize) -> Self
        where T: PageBoxable
    {
        // Ensure that the page address is aligned to the page size.
        assert!((page_address % PAGE_SIZE) == 0,
                "Page address must be aligned to the page size ({} bytes).",
                PAGE_SIZE);

        let mut pointer = VirtualPagePtr::new_from_address(page_address)
            .expect("Failed to create a virtual page pointer from the physical address.");

        unsafe
        {
            // Allow the type to initialize itself in the page of memory.
            T::init_in_place(&mut pointer);
        }

        Self { pointer }
    }
}



impl<T> Deref for PageBox<T>
{
    type Target = T;

    /// Dereference the `PageBox` to get a reference to the contained type.
    fn deref(&self) -> &Self::Target
    {
        &*self.pointer
    }
}



impl<T> DerefMut for PageBox<T>
{
    /// Dereference the `PageBox` to get a mutable reference to the contained type.
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        &mut *self.pointer
    }
}



impl<T: ?Sized> Drop for PageBox<T>
{
    /// Drop the `PageBox` and free the page of memory it was using.
    fn drop(&mut self)
    {
        unsafe
        {
            let page_address = usize::from(&self.pointer);

            drop_in_place(self.pointer.as_mut_ptr());
            free_page(page_address);
        }
    }
}
