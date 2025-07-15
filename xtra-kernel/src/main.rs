
// The main entry point for the Xtra kernel. We perform all system initialization here and then
// jump into the scheduler to start running tasks.

#![no_std]
#![no_main]
#![feature(let_chains)]



// extern crate alloc;


// Bring in the kernel subsystems that implement the core functionality of the Xtra kernel.
mod riscv;
mod device_tree;
mod uart;
mod printing;
mod filesystems;
mod scheduler;



use core::{ arch::naked_asm, panic::PanicInfo };

use crate::{ device_tree::DeviceTree, printing::init_printing, scheduler::Scheduler };



// The OS banner to print at startup, this is a simple ASCII art banner that is printed to the
// UART console when the bootloader starts.
const OS_BANNER_STR: &str = include_str!("../banner.txt");


// A banner for the OS panic message when printed out the UART console.
const OS_PANIC_STR: &str = include_str!("../panic.txt");



// This is the raw starting point of the bootloader, it is called directly by the host environment,
// in this case, QEMU. We setup a reasonable stack pointer and then jump to the main function, we
// expect main to never return as it is its job to find and load the actual kernel image and
// transfer control to it.
#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text._start"]
pub unsafe extern "C" fn _start()
{
    // This function is called system startup code. There is no Rust runtime available at this
    // point, so we cannot use any Rust features, we just setup the stack and then jump to the
    // proper main function.
    naked_asm!
    (
        //"la sp, _stack_start", // Load the stack pointer from the linker script.
        "j main"               // hart_id and dtb are already in a0 and a1, so just call main.
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
pub extern "C" fn main(_hart_id: usize, device_tree_ptr: *const u8) -> !
{
    // Initialize the device tree iterator from the pointer passed in by the host environment.
    let device_tree = DeviceTree::new(device_tree_ptr);

    // Init the logging system using the device tree to find the UART device. We use the system's
    // first UART device for system logging. Any other UART devices will be used as consoles.
    init_printing(&device_tree);

    // Print the OS banner to the UART console.
    println!("{}", OS_BANNER_STR);

    // Finally initialize the scheduler for this CPU core and start it running. The scheduler's run
    // method will never return.
    let scheduler = Scheduler::new();

    scheduler.run();
}
