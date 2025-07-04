
#![no_std]
#![no_main]
#![allow(unused)]
#![feature(let_chains)]



mod uart;
mod power;
mod device_tree;
mod ram;
mod block_device;
mod fat32;
mod elf;



use core::{ arch::naked_asm, panic::PanicInfo };

use uart::{ Uart, UART_0_BASE };
use power::{ power_off, wait_for_interrupt };
use device_tree::{ DeviceTree, validate_dtb };



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
        "la sp, _stack_start", // Load the stack pointer from the linker script.
        "j main"               // hart_id and dtb are already in a0 and a1, so just call main.
    );
}


#[panic_handler]
fn kernel_panic_handler(info: &PanicInfo) -> !
{
    // Get a reference to the UART, we will use it to print the panic message. Note that we assume
    // that the UART is already initialized at this point, so we don't try to initialize it again.
    let uart = Uart::new(UART_0_BASE);

    uart.put_str("\n\nBoot-Loader panic occurred!\n");

    // Let the user know the location of the panic, if available.
    if let Some(location) = info.location()
    {
        uart.put_str("Panic occurred at: ");
        uart.put_str(location.file());
        uart.put_str(":");
        uart.put_int(location.line() as usize);
        uart.put_str("\n");
    }

    // Let the user know that we are shutting down the system.
    uart.put_str("\nSystem will now power off...\n");
    power_off();
}


fn write_startup_banner(uart: &uart::Uart, hart_id: usize, device_tree_ptr: *const u8)
{
    // Write the welcome message.
    uart.put_str("\n\nXTRA-OS Bootloader Starting...\n");

    // Let the user know which hart (hardware thread) is running this code.
    uart.put_str("Running on hart ID: ");
    uart.put_int(hart_id);
    uart.put_str("\n");

    // Write the address of the Device Tree Blob (DTB) pointer.
    uart.put_str("Device Tree Blob (DTB) address: ");
    uart.put_hex(device_tree_ptr as usize, true);
    uart.put_str("\n");
}


fn validate_device_tree(uart: &uart::Uart, device_tree_ptr: *const u8)
{
    // Validate the Device Tree Blob (DTB) by checking its magic number.
    if !validate_dtb(device_tree_ptr)
    {
        uart.put_str("Invalid Device Tree Blob (DTB) magic number!\n");
        uart.put_str("Shutting down system...\n");

        power_off();
    }

    uart.put_str("Device Tree Blob (DTB) is valid!\n");
}


#[no_mangle]
pub extern "C" fn main(hart_id: usize, device_tree_ptr: *const u8) -> !
{
    // Check to make sure that we are running on the boot hart (hart_id 0).
    if hart_id != 0
    {
        // We're not, so we will wait in an idle state.
        unsafe
        {
            wait_for_interrupt();
        }
    }

    // Initialize the UART for logging, and then log the bootloader start message.
    let uart = Uart::init_new(UART_0_BASE);

    write_startup_banner(&uart, hart_id, device_tree_ptr);

    // Validate the DTB, if the DTB is invalid, we will print an error message and shut down the
    // system.
    validate_device_tree(&uart, device_tree_ptr);

    // We seem to have a valid DTB, so let's print the information we've found for diagnostics.
    let device_tree = DeviceTree::new(device_tree_ptr);

    uart.put_str("\n");
    device_tree.print_tree(&uart);

    // Find the first bootable block device.

    // Take the boot device find a bootable partition.

    // Read the partition table and find the kernel image. We will then:
    //     * Validate the kernel image.
    //     * Read it's memory requirements.
    //     * Find it's entry point.

    // Get information about the system RAM and compute a loading address for the kernel.
    // Compute the kernel's final entry point address.

    // Load the kernel image into memory.

    // Jump to the kernel's entry point, passing the hart ID and DTB pointer as arguments.

    // If we get here something went wrong, so we will always just power off the system.
    uart.put_str("\nBootloader erroneously returned to, powering off system...\n");
    power_off()
}
