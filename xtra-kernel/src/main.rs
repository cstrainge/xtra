
// The main entry point for the Xtra kernel. We perform all system initialization here and then
// jump into the scheduler to start running tasks.

#![no_std]
#![no_main]
#![feature(let_chains)]



// extern crate alloc;



// Bring in the kernel subsystems that implement the core functionality of the Xtra kernel.



/// The architecture specific code for the kernel. This will contain the low level code that is
/// specific to the architecture we are running on, in this case, RISC-V.
mod arch;

/// The simple logging UART device handler. This version of the UART doesn't handle input or
/// interrupts,
mod uart;

// Because this is a no_std environment we define our own implementations of the print! and println!
// macros here.
#[macro_use]
mod printing;

/// All of the locking primitives used in the kernel.
mod locking;

/// The memory management for the kernel. This includes raw page management and virtualization, as
/// well as the heap allocator for the kernel built atop of the page allocator.
mod memory;

/// The file system support for the kernel. Including our implementation of FAT-32 and Ext2 file
/// systems.
mod filesystems;

/// The scheduler for the kernel. It's here where we manage all of the user processes and their
/// threads.
mod scheduler;



use core::{ arch::naked_asm,
            hint::spin_loop,
            panic::PanicInfo,
            ptr::addr_of_mut,
            sync::atomic::{ AtomicBool, Ordering } };

use crate::{ arch::{ device_tree::DeviceTree, get_core_index, print_cpu_info },
             printing::init_printing,
             memory::{ kernel::KernelMemoryLayout,
                       memory_device::SystemMemory,
                       mmu::{ convert_to_kernel_address_space, init_memory_manager } },
             scheduler::Scheduler };



/// The OS banner to print at startup, this is a simple ASCII art banner that is printed to the
/// UART console when the bootloader starts.
const OS_BANNER_STR: &str = include_str!("../banner.txt");

/// A banner for the OS panic message when printed out the UART console.
const OS_PANIC_STR: &str = include_str!("../panic.txt");

/// The version of the kernel, this is used to identify the kernel version in logs and other output.
const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The time the kernel was built.
const KERNEL_BUILD_TIME: &str = env!("BUILD_TIME");

/// The profile the kernel was built with.
const KERNEL_PROFILE: &str = env!("PROFILE");

/// Go with a 4KB stack size for each CPU core.
///
/// TODO: Move this into arch and make it a configurable option in the kernel config file.
///
/// **WARNING**: This **needs** to be kept in sync with the linker script as it defines the size of
///              the .stacks section in the kernel binary layout. If we change it here we need to
///              change it in the linker script as well.
const STACK_SIZE: usize = 0x1000;

/// Maximum number of cores we support in the system.
///
/// TODO: Move this into arch and make it a configurable option in the kernel config file.
///
/// **WARNING**: This **needs** to be kept in sync with the linker script as it defines the size of
///              the .stacks section in the kernel binary layout. If we change it here we need to
///              change it in the linker script as well.
const MAX_CORES: usize = 4;

/// Allocate the space for a stack for each core in the system.
///
/// TODO: Move this into arch and make it a configurable option in the kernel config file.
#[no_mangle]
#[link_section = ".stacks"]
static mut STACKS: [u8; STACK_SIZE * MAX_CORES] = [0; STACK_SIZE * MAX_CORES];



/// Keep track of whether the system has booted or not. This is used to ensure that only the first
/// hart runs the boot process and the others wait for it to complete.
static mut SYSTEM_BOOTED: AtomicBool = AtomicBool::new(false);



/// Check if the system has booted yet. This is used to ensure that only the first hart runs the
/// boot process and the others wait for it to complete.
fn system_booted() -> bool
{
    let booted_flag = unsafe { &mut *addr_of_mut!(SYSTEM_BOOTED) };

    booted_flag.load(Ordering::Acquire)
}



/// Signal to the secondary harts that the system is ready for them to start running their scheduler.
/// This is called by the first hart after it has completed the boot process.
fn set_system_booted()
{
    let booted_flag = unsafe { &mut *addr_of_mut!(SYSTEM_BOOTED) };

    booted_flag.store(true, Ordering::Release);
}



// TODO: Move this into the arch module as it is architecture specific.
/// This is the raw starting point of the bootloader, it is called directly by the host environment,
/// in this case, QEMU. We setup a reasonable stack pointer and then jump to the main function, we
/// expect main to never return as it is its job to find and load the actual kernel image and
/// transfer control to it.
#[cfg(target_arch = "riscv64")]
#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text._start"]
pub unsafe extern "C" fn _start() -> !
{
    // This function is called system startup code. There is no Rust runtime available at this
    // point, so we cannot use any Rust features, we just setup the stack and then jump to the
    // proper main function.
    naked_asm!
    (
        // a0 = hart_id, a1 = dtb_ptr
        "la t0, STACKS",        // t0 = &STACKS.
        "li t1, {stack_size}",  // t1 = STACK_SIZE.
        "mul t2, a0, t1",       // t2 = hart_id * STACK_SIZE.

        "add t0, t0, t2",       // t0 = &STACKS[hart_id * STACK_SIZE].
        "add t0, t0, t1",       // t0 = &STACKS[(hart_id+1)*STACK_SIZE].
        "mv sp, t0",            // set sp to top of stack for this hart.

        "j main",               // hart_id and dtb are already in a0 and a1, so just call main.

        stack_size = const STACK_SIZE
    );
}



/// This is the panic handler for the kernel, it is called when a panic occurs in the kernel code.
/// We print the panic message to the UART console and then loop forever.
#[panic_handler]
fn kernel_panic_handler(info: &PanicInfo) -> !
{
    // TODO: If println has not been initialized yet, we should attempt to do so here.
    // TODO: Halt the other harts and disable interrupts.

    let core_index = get_core_index();

    println!("{}", OS_PANIC_STR);
    println!("Fatal error occurred on core {:02}: {}", core_index, info);

    // TODO: Restart the system gracefully, if possible.
    loop
    {
        // Spin forever, we cannot recover from a panic in the kernel.
        spin_loop();
    }
}



/// The main entry point for the kernel, this function will never return.  Either it runs forever or
/// a shutdown is initiated.
#[no_mangle]
pub extern "C" fn main(core_index: usize, device_tree_ptr: *const u8) -> !
{
    // Make sure that we can support the number of cores we have in the system.
    assert!(core_index < MAX_CORES,
            "Unsupported CPU hart ID: {:02}, max supported cores: {:02}.",
            core_index,
            MAX_CORES);

    // Make sure that the core index matches the one supplied by the bootloader. If it isn't then
    // something seriously wrong has happened.
    assert!(core_index == get_core_index(),
            "Boot supplied Hart ID {:02} does not match current core index {:02}.",
            core_index,
            get_core_index());

    // Make sure that we are only running the core boot process on the first hart.
    if core_index != 0
    {
        // Wait for the boot process to complete.
        while !system_booted()
        {
            // Let the compiler know that this is a busy wait. This will allow it to emit hints to
            // the CPU to optimize this loop and minimize it's power usage.
            spin_loop();
        }

        // Let the world know we're running.
        println!("HCore {:02} is now running.", core_index);

        // We know that the memory manager has been initialized by the first hart, so we can safely
        // switch to the kernel address space and start running the scheduler.
        println!("Switching to kernel address space for hart {:02}.", core_index);

        convert_to_kernel_address_space();
    }
    else
    {
        // Initialize the device tree iterator from the pointer passed in by the host environment.
        let device_tree = DeviceTree::new(device_tree_ptr);

        // Init the logging system using the device tree to find the UART device. We use the
        // system's first UART device for system logging. Any other UART devices will be used as
        //  consoles.
        init_printing(&device_tree);

        // Print the OS banner to the UART console.
        print!("{}", OS_BANNER_STR);
        println!("Kernel version:      {}", KERNEL_VERSION);
        println!("Kernel build time:   {}", KERNEL_BUILD_TIME);
        println!("Kernel profile:      {}", KERNEL_PROFILE);
        println!();

        // Print out the CPU information for the current core.
        print_cpu_info();

        // Determine where in RAM the kernel is loaded. We need to keep track of this so that we can
        // mark these pages as used in the memory manager.
        let kernel_memory_layout = KernelMemoryLayout::new();

        println!("{}", kernel_memory_layout);

        // Interrogate the memory to find out what we are working with.
        let memory_info = SystemMemory::new(&device_tree);

        println!("{}", memory_info);

        // We now need to properly initialize the MMU and map the kernel into high memory so that we
        // can run from our proper address. This will involve resetting the PC to the new kernel
        // address space.
        println!("Initializing memory manager...");

        init_memory_manager(&kernel_memory_layout, &memory_info)
            .expect("Failed to initialize memory manager");

        convert_to_kernel_address_space();

        // Now make sure that MMIO pages are mapped correctly so that we can access the hardware
        // devices. We also need to make sure those pages are marked as used in the memory manager.

        // Now we can initialize our heap allocator so that it can manage our heap memory in our
        // proper address space.

        // Initialize the interrupt controller so that we can handle interrupts and exceptions in
        // the kernel.

        // Walk the device tree and find and initialize our supported devices. Once this is done we
        // can free the device tree pages. Any information needed from the device tree should be
        // copied by the respective device drivers.

        // Now that we have all the devices initialized, we can initialize the file systems and
        // mount the root file system. We will need to find the boot volume and find the partition
        // mapping so that we can map all partitions to where they need to go.

        // At this point we can start process 0, the idle process. If there is no other process that
        // can be run at any given time, the idle process will run. This is a simple process
        // that just spins and does nothing. It is used to keep the CPU busy when there are no
        // other processes to run. This is useful for power management. The CPU can safely run at a
        // lower frequency and power state when it is running the idle process.

        // We have a root file system at this point, we can now look under /bin and find the init
        // program and prepare it for execution.

        // Let other harts know that the boot process is complete.
        set_system_booted();
    }

    // Finally initialize the scheduler for this CPU core and start it running. The scheduler's run
    // method will never return.
    //
    // This will allow init to run and it will take care of the rest of the boot sequence and get us
    // to a running system.
    println!("Starting scheduler for hart {}.", core_index);

    let scheduler = Scheduler::new();

    scheduler.run();
}
