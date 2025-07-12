
// Bootloader for XTRA-OS
//
// This bootloader is designed to run on RISC-V systems, it initializes the UART for logging,
// validates the Device Tree Blob (DTB), and prepares the system to load and run a kernel image.
//
// The Rust code is designed to run in a minimal environment without the Rust standard library or a
// working heap. So no memory allocation is available, except for the stack which is set up by the
// linker script and startup code in _start().
//
// Right now it is designed to run on QEMU, in the future it may be extended to run on real RISC-V
// hardware. In order to support this we need to get the UART and reset device information from the
// Device Tree Blob (DTB) that is passed to the bootloader by the host environment.
//
// Currently we've baked in assumptions about this hardware.
//
// This bootloader is the first code that runs on the system, it is responsible for:
//     * Initializing the UART for logging.
//     * Validating the Device Tree Blob (DTB).
//     * Finding and loading a kernel image from a block device, (on a fat32 partition).
//
// So to summarize, the key assumptions of this bootloader are:
//
//  - Runs as the first code after firmware (no other OS/bootloader runs before us).
//  - UART MMIO base address is known/fixed (see UART_0_BASE), but future versions may parse this
//    from the DTB.
//  - Only the stack is available for runtime allocations (no heap.)
//  - Device Tree Blob (DTB) is passed in as an argument from the host/firmware.
//  - Block device assumed to be VirtIO-MMIO, FAT32, QEMU default, but will generalize in the
//    future.
//  - Kernel image is an ELF file called "kernel.elf" stored in the root of a fat32 partition.
//  - Bootloader region may be overwritten after handoff to the kernel.



#![no_std]
#![no_main]
#![allow(unused)]
#![feature(let_chains)]



mod uart;
mod power;
mod device_tree;
mod virtio;
mod block_device;
mod partition_table;
mod fat32;
mod ram;
mod elf;



use core::{ arch::naked_asm, panic::PanicInfo };

use uart::{ Uart, UART_0_BASE };
use power::{ power_off, wait_for_interrupt };
use device_tree::{ DeviceTree, validate_dtb };
use block_device::BlockDevice;

use crate::{fat32::{DirectoryEntry, DirectoryIterator, Fat32Volume}, virtio::SECTOR_SIZE};



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


// This is a fairly simple panic handler that will be called if a panic occurs in the bootloader.
// We can't currently print out the reason for the panic because the formatting code requires a
// working heap, which we don't have in the bootloader. So we will just print the location of the
// panic, if available, and then power off the system.
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


// Write our startup banner to the UART. This will include a welcome message, the hart ID, and the
// address of the Device Tree Blob (DTB) pointer.
//
// This is mostly for diagnostic purposes, so that we can see which hart is running the bootloader
// and the address of the DTB that was passed to it.
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


// Validate the Device Tree Blob (DTB) by checking its magic number. The magic number is a unique
// identifier that indicates the start of a valid DTB.
//
// If we don't find a proper device tree we will print an error message and shut down the system.
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


// The actual Rust level entry point for the bootloader. This function is called indirectly by the
// host environment to manage the boot process.
//
// Here we initialize the UART 0 for logging, validate the Device Tree Blob (DTB), and print out
// the information we find in the DTB for diagnostics.
//
// Then we continue on with the boot process, which will involve finding a bootable block device,
// with a fat32 partition with a kernel.elf file on it. We will then read the kernel image,
// validate it, and load it into memory. Finally we will jump to the kernel's entry point, passing
// the hart ID and DTB pointer as arguments.
//
// It is expected that this function wi9ll never return, but also that it will never be returned to
// by the kernel. It is the job of the kernel to take over control of the system and manage the
// hardware from that point on.
//
// In fact it is expected that this bootloader code will be overwritten later by normal OS
// operation.
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
    let block_device = BlockDevice::find_first_drive(&uart, device_tree);

    if block_device.is_none()
    {
        uart.put_str("\nNo bootable block device found!\n");
        uart.put_str("Shutting down system...\n");

        power_off();
    }

    // Take the boot device find a bootable partition.
    let mut block_device = block_device.unwrap();

    block_device.initialize(&uart);

    let partition = block_device.find_bootable_partition(&uart);

    if partition.is_none()
    {
        uart.put_str("\nNo bootable partition found on block device!\n");
        uart.put_str("Shutting down system...\n");

        power_off();
    }

    let partition = partition.unwrap();

    uart.put_str("Partition information:\n");
    uart.put_str("  Is FAT:          ");
    uart.put_str(if partition.is_fat() { "Yes" } else { "No" });
    uart.put_str("\n");
    uart.put_str("  Is bootable:     ");
    uart.put_str(if partition.is_bootable() { "Yes" } else { "No" });
    uart.put_str("\n");
    uart.put_str("  Start LBA:       ");
    uart.put_int(partition.start_lba as usize);
    uart.put_str("\n");
    uart.put_str("  Size in sectors: ");
    uart.put_int(partition.size_in_sectors as usize);
    uart.put_str(", ");
    uart.put_int(partition.size_in_sectors as usize * SECTOR_SIZE);
    uart.put_str(" bytes.\n");
    uart.put_str("\n");
    uart.put_str("Reading FAT32 partition...\n");

    // Initialize the fat32 volume for reading.
    let fat32_volume = Fat32Volume::new(&block_device, &partition);

    if let Err(e) = fat32_volume
    {
        uart.put_str("Failed to initialize FAT32 volume.\n");
        uart.put_str("Error: ");
        uart.put_str(e);
        uart.put_str("\n");

        power_off();
    }

    // Now that we have a valid FAT32 volume, we can create a directory iterator for the root
    // directory of the volume.
    let fat32_volume = fat32_volume.unwrap();
    let directory_iterator = DirectoryIterator::new(&fat32_volume, fat32_volume.root_cluster);

    // Was the directory iterator initialized successfully?
    if let Err(e) = directory_iterator
    {
        uart.put_str("Failed to initialize directory iterator.\n");
        uart.put_str("Error: ");
        uart.put_str(e);
        uart.put_str("\n");

        power_off();
    }

    let mut directory_iterator = directory_iterator.unwrap();

    // Iterate over the entries in the root directory, looking for a file called "kernel.elf".
    uart.put_str("Searching for kernel image in root directory...\n");

    let mut kernel_entry = DirectoryEntry::zeroed();

    let result = directory_iterator.iterate(|entry|
        {
            if    entry.is_file()
               && entry.name == *b"KERNEL  ELF"
            {
                uart.put_str("Found OS kernel, the file is ");
                uart.put_int(entry.file_size as usize);
                uart.put_str(" bytes.\n");

                // We found the kernel image, so we will return it.
                kernel_entry = entry.clone();

                false
            }
            else
            {
                true
            }
        });

    if let Err(e) = result
    {
        uart.put_str("Failed to iterate over root directory.\n");
        uart.put_str("Error: ");
        uart.put_str(e);
        uart.put_str("\n");

        power_off();
    }

    // Get information about the system RAM and compute a loading address for the kernel.
    // Compute the kernel's final entry point address.

    // Load the kernel image into memory.

    // Jump to the kernel's entry point, passing the hart ID and DTB pointer as arguments.

    // If we get here something went wrong, so we will always just power off the system.
    uart.put_str("\nExecution erroneously returned to the bootloader, powering off system...\n");

    power_off()
}
