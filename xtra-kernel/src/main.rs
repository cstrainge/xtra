
// The main entry point for the Xtra kernel. We perform all system initialization here and then
// jump into the scheduler to start running tasks.

#![no_std]
#![no_main]
#![feature(let_chains)]



// extern crate alloc;



// Bring in the kernel subsystems that implement the core functionality of the Xtra kernel.
#[cfg(target_arch = "riscv64")]
mod riscv;

// Make the printing macros available globally in the kernel.
#[macro_use]
mod printing;

mod device_tree;
mod uart;
mod memory;
mod filesystems;
mod scheduler;



use core::{ arch::naked_asm,
            fmt::Write,
            hint::spin_loop,
            panic::PanicInfo,
            ptr::addr_of_mut,
            sync::atomic::{ AtomicBool, Ordering } };

use crate::{ device_tree::DeviceTree,
             printing::init_printing,
             memory::{ kernel::KernelMemoryLayout, memory_device::SystemMemory },
             scheduler::Scheduler };



// The OS banner to print at startup, this is a simple ASCII art banner that is printed to the
// UART console when the bootloader starts.
const OS_BANNER_STR: &str = include_str!("../banner.txt");


// A banner for the OS panic message when printed out the UART console.
const OS_PANIC_STR: &str = include_str!("../panic.txt");


// The version of the kernel, this is used to identify the kernel version in logs and other output.
const KERNEL_VERSION: &str = env!("CARGO_PKG_VERSION");


// The time the kernel was built.
const KERNEL_BUILD_TIME: &str = env!("BUILD_TIME");


// The profile the kernel was built with.
const KERNEL_PROFILE: &str = env!("PROFILE");


// Keep track of whether the system has booted or not. This is used to ensure that only the first
// hart runs the boot process and the others wait for it to complete.
static mut SYSTEM_BOOTED: AtomicBool = AtomicBool::new(false);



const STACK_SIZE: usize = 0x1000;  // Go with a 4KB stack size for each hart.
const MAX_HARTS: usize = 4;        // Maximum number of harts we support in the system.



// Allocate the space for a stack for each hart in the system.
// TODO: Make this a configurable option in a kernel config file.
#[no_mangle]
#[link_section = ".stacks"]
static mut STACKS: [u8; STACK_SIZE * MAX_HARTS] = [0; STACK_SIZE * MAX_HARTS];



// Check if the system has booted yet. This is used to ensure that only the first hart runs the
// boot process and the others wait for it to complete.
fn system_booted() -> bool
{
    let booted_flag = unsafe { &mut *addr_of_mut!(SYSTEM_BOOTED) };

    booted_flag.load(Ordering::Acquire)
}



// Signal to the secondary harts that the system is ready for them to start running their scheduler.
// This is called by the first hart after it has completed the boot process.
fn set_system_booted()
{
    let mut booted_flag = unsafe { &mut *addr_of_mut!(SYSTEM_BOOTED) };

    booted_flag.store(true, Ordering::Release);
}



// This is the raw starting point of the bootloader, it is called directly by the host environment,
// in this case, QEMU. We setup a reasonable stack pointer and then jump to the main function, we
// expect main to never return as it is its job to find and load the actual kernel image and
// transfer control to it.
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



// This is the panic handler for the kernel, it is called when a panic occurs in the kernel code.
// We print the panic message to the UART console and then loop forever.
//
// TODO: Add a timeout and attempt to power off the system gracefully.
#[panic_handler]
fn kernel_panic_handler(info: &PanicInfo) -> !
{
    println!("{}", OS_PANIC_STR);
    println!("Kernel panic: {}", info);

    loop {}
}


#[no_mangle]
pub extern "C" fn main(hart_id: usize, device_tree_ptr: *const u8) -> !
{
    // Make sure that we are only running the core boot process on the first hart.
    if hart_id != 0
    {
        // Wait for the boot process to complete.
        while !system_booted()
        {
            // Let the compiler know that this is a busy wait. This will allow it to emit hints to
            // the CPU to optimize this loop and minimize it's power usage.
            spin_loop();
        }
    }
    else
    {
        // Initialize the device tree iterator from the pointer passed in by the host environment.
        let device_tree = DeviceTree::new(device_tree_ptr);

        // Get our base CPU information from the RISC-V CSR registers.
        let vendor_id = crate::riscv::csr::read_mvendorid();
        let arch_id = crate::riscv::csr::read_marchid();
        let imp_id = crate::riscv::csr::read_mimpid();

        // Init the logging system using the device tree to find the UART device. We use the
        // system's first UART device for system logging. Any other UART devices will be used as
        //  consoles.
        init_printing(&device_tree);

        // Print the OS banner to the UART console.
        print!("{}", OS_BANNER_STR);
        println!("Kernel version:    {}", KERNEL_VERSION);
        println!("Kernel build time: {}", KERNEL_BUILD_TIME);
        println!("Kernel profile:    {}", KERNEL_PROFILE);
        println!();

        println!("CPU Information:");
        println!("Vendor ID:         0x{:x}", vendor_id);
        println!("Arch ID:           0x{:x}", arch_id);
        println!("Implementation ID: 0x{:x}", imp_id);
        println!("Hart ID:           {}", hart_id);
        println!();

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
    let scheduler = Scheduler::new();

    scheduler.run();
}
