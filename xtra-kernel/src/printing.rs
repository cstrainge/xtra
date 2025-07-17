
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



// Function to format a number as a comma-separated string.
pub fn comma_separated_int(number: u64, buffer: &mut [u8; 32]) -> usize
{
    let mut number = number;
    let mut index = buffer.len() - 1;
    let mut count = 0;

    buffer.fill(0);

    if number == 0
    {
        buffer[index] = b'0';
        return index;
    }

    while number > 0
    {
        // Make sure to add the comma every 2 digits.
        if    count > 0
           && count % 3 == 0
        {
            buffer[index] = b',';
            index -= 1;
        }

        buffer[index] = (number % 10) as u8 + b'0';
        number /= 10;
        count += 1;

        if index == 0
        {
            break;
        }

        index -= 1;
    }

    index + 1
}



// Function to format a floating-point number as a comma-separated string.
pub fn comma_separated_float(number: f64, buffer: &mut [u8; 64]) -> usize
{
    let mut integer_buffer = [0u8; 32];

    // Simple integer conversion
    let integer_part = number as u64;
    let fractional_part = ((number - integer_part as f64) * 10.0 + 0.5) as u64;  // ✅ Manual rounding

    // Handle case where rounding pushes us to next integer (e.g., 9.95 -> 10.0)
    let (final_integer, final_fractional) = if fractional_part >= 10 {
        (integer_part + 1, 0)
    } else {
        (integer_part, fractional_part)
    };

    let integer_start = comma_separated_int(final_integer, &mut integer_buffer);
    let integer_len = 32 - integer_start;

    buffer[0..integer_len].copy_from_slice(&integer_buffer[integer_start..]);
    buffer[integer_len] = b'.';
    buffer[integer_len + 1] = final_fractional as u8 + b'0';

    integer_len + 2
}



#[macro_export]
macro_rules! buffer_as_string
{
    ($buffer:expr) => {{
        use core::str;

        // SAFETY: We assume the buffer is valid UTF-8.
        unsafe { str::from_utf8_unchecked(&$buffer) }
    }};
}



// Format a data size in a human-readable format, e.g., "1.2 MB (1,234,567 bytes)".
#[macro_export]
macro_rules! write_size
{
    ($f:expr, $n:expr) =>
        {{
            use crate::printing::{ comma_separated_float, comma_separated_int };

            let mut float_buffer = [0u8; 64];
            let mut int_buffer = [0u8; 32];

            let n = $n as u64;

            if n >= 1_048_576 * 1024
            {
                let float_value = n as f64 / (1_048_576.0 * 1024.0);
                let float_length = comma_separated_float(float_value, &mut float_buffer);
                let float_string = buffer_as_string!(&float_buffer[..float_length]);

                let int_start = comma_separated_int(n, &mut int_buffer);
                let int_string = buffer_as_string!(&int_buffer[int_start..]);

                write!($f, "{} GB ({} bytes)",
                        float_string,
                        int_string)
            }
            else if n >= 1_048_576
            {
                let float_value = n as f64 / 1_048_576.0;
                let float_length = comma_separated_float(float_value, &mut float_buffer);
                let float_string = buffer_as_string!(&float_buffer[..float_length]);

                let int_start = comma_separated_int(n, &mut int_buffer);
                let int_string = buffer_as_string!(&int_buffer[int_start..]);

                write!($f, "{} MB ({} bytes)",
                        float_string,
                        int_string)
            }
            else if n >= 1024
            {
                let float_value = n as f64 / 1024.0;
                let float_length = comma_separated_float(float_value, &mut float_buffer);
                let float_string = buffer_as_string!(&float_buffer[..float_length]);

                let int_start = comma_separated_int(n, &mut int_buffer);
                let int_string = buffer_as_string!(&int_buffer[int_start..]);

                write!($f, "{} KB ({} bytes)", float_string, int_string)
            }
            else
            {
                let int_start = comma_separated_int(n, &mut int_buffer);
                let int_string = buffer_as_string!(&int_buffer[int_start..]);

                write!($f, "{} bytes", int_string)
            }
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
