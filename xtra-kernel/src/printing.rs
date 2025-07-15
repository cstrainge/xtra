
// Implementation of the printing module for the Xtra kernel.
//
// This module provides a simple logging interface to the first UART device found in the device
// tree. We use the simple UART implementation so that we can print from code executing  without
// interrupts enabled.

use crate::{ device_tree::DeviceTree, uart::SimpleUart };



// Global reference to the UART device used for printing. This is initialized at boot time and
// used throughout the kernel for logging output.
pub static mut PRINTING_UART: SimpleUart = SimpleUart::zeroed();



// Implement the standard print! macro for printing formatted output from the kernel to an attached
// UART device.
#[macro_export]
macro_rules! print
{
    ($($arg:tt)*) =>
        {{
            use core::{ fmt::Write, ptr::addr_of_mut };
            use crate::printing::PRINTING_UART;

            unsafe
            {
                // When printing we print to the logging UART device that we initialized at boot
                // time.
                let uart = &mut *addr_of_mut!(crate::printing::PRINTING_UART);

                if uart.is_initialized()
                {
                    uart.write_fmt(format_args!($($arg)*)).unwrap();
                }
            }
        }};
}



// Implement the standard println! macro for printing formatted output from the kernel to an
// attached UART device. This macro appends a newline character to the end of the output.
#[macro_export]
macro_rules! println
{
    () =>
        {{
            print!("\n");
        }};

    ($fmt:expr) =>
        {{
            print!(concat!($fmt, "\n"));
        }};

    ($fmt:expr, $($arg:tt)*) =>
        {{
            print!(concat!($fmt, "\n"), $($arg)*);
        }};
}



// Initializes the printing system by finding the first UART device in the device tree and setting
// it up for use. This function will panic if no UART device is found in the device tree.
pub fn init_printing(device_tree: &DeviceTree)
{
    let mut found_uart = false;

    device_tree.iterate_blocks(|offset, name|
        {
            // Extract the device name from the tree node name.
            let device_name = if let Some(at_index) = name.find('@')
                {
                    &name[..at_index]
                }
                else
                {
                    name
                };

            // Is this a UART device? If so, we will initialize the logging UART.
            if device_name == "serial"
            {
                let mut base_address: u64 = 0;
                let mut reg_range: u64 = 0;

                device_tree.iterate_properties(offset, |prop_name, prop_value|
                    {
                        if prop_name == "reg"
                        {
                            if prop_value.len() < 16
                            {
                                // Invalid 'reg' property length, we expect at least 16 bytes.
                                // Bail from this device's properties.
                                return false;
                            }

                            let base_bytes = prop_value[0..8].try_into().unwrap();
                            let range_bytes = prop_value[8..16].try_into().unwrap();

                            base_address = u64::from_be_bytes(base_bytes);
                            reg_range = u64::from_be_bytes(range_bytes);
                        }

                        true
                    });

                if    base_address != 0
                   && reg_range != 0
                {
                    unsafe
                    {
                        PRINTING_UART = SimpleUart::init_new(base_address as usize);
                    }

                    found_uart = true;

                    return false;
                }
            }

            // Continue iterating, we haven't found our UART device yet.
            true
        });

    if !found_uart
    {
        panic!("No UART device found in the device tree for logging.");
    }
}
