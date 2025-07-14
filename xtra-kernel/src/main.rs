
#![no_std]
#![no_main]
#![feature(let_chains)]



use core::{ arch::naked_asm, panic::PanicInfo };



// Import the UART module for console output.  This is just a temporary implementation imported from
// the bootloader. We will be building proper infrastructure for this in the future.
mod uart;



// The OS banner to print at startup, this is a simple ASCII art banner that is printed to the
// UART console when the bootloader starts.
const OS_BANNER: &str = include_str!("../banner.txt");



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


#[panic_handler]
fn kernel_panic_handler(_info: &PanicInfo) -> !
{
    let uart = uart::Uart::new(uart::UART_0_BASE);

    uart.put_str("Kernel panic occurred.\n");

    loop {}
}


#[no_mangle]
pub extern "C" fn main(_hart_id: usize, _device_tree_ptr: *const u8) -> !
{
    let uart = uart::Uart::init_new(uart::UART_0_BASE);

    // Print the OS banner to the UART console so that we know the kernel is alive.
    uart.put_str(OS_BANNER);

    loop {}
}
