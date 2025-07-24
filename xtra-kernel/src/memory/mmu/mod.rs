
/// High level memory page management for the kernel.

use crate::memory::{ kernel::KernelMemoryLayout, memory_device::SystemMemory };



//
mod free_page_list;

use free_page_list::init_free_page_list;




pub trait PageTable
{
}



pub trait SystemMmu
{
}



/// Initialize the system's memory management unit, (MMU,) and the higher level data strictures
/// around it.
///
/// The page tables for the kernel itself will be initialized and the free page list allocated and
/// prepared for allocating pages of memory for the kernel and user processes.
///
/// We also zero initialize all the free pages in the system so that we can safely use them for any
/// purpose.
pub fn init_memory_manager(kernel_memory: &KernelMemoryLayout,
                           system_memory: &SystemMemory) -> Result<(), &'static str>
{
    // Initialize the free page list. Now we will be able to keep track of the free pages in the
    // system. We make sure to not allocate any pages that are part of the kernel's memory layout
    // and also avoid allocating pages that belong to MMIO devices.
    init_free_page_list(kernel_memory, system_memory);

    Ok(())
}



/// This function will switch from the raw address space to the virtual address space of the kernel
/// this will map the kernel into high memory and switch the MMU to use the kernel's page tables as
/// initialized earlier by the memory manager's initialization function.
///
/// This will also adjust the kernel's stack pointer and reset the PC to the new virtual address
/// space of the kernel.
///
/// THis function will panic on failure.
pub fn convert_to_kernel_address_space()
{
    panic!("Switching to kernel address space is not implemented yet.");
}
