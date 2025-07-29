
// Implementation of the printing module for the Xtra kernel.
//
// This module provides a simple logging interface to the first UART device found in the device
// tree. We use the simple UART implementation so that we can print from code executing  without
// interrupts enabled.

use core::fmt::{ self, Write };

use crate::{ arch::device_tree::DeviceTree,
             locking::spin_lock::SpinLock,
             uart::SimpleUart };



/// Global reference to the UART device used for printing. This is initialized at boot time and
/// used throughout the kernel for logging output.
pub static mut PRINTING_UART: SimpleUart = SimpleUart::zeroed();



/// Spinlock for protecting access to the printing UART. This is used to ensure that only one thread
/// can write text out the UART at a time, preventing interleaved output.
pub static PRINTING_LOCK: SpinLock = SpinLock::new();



/// A simple writer that writes to a buffer. This is used to format strings using the `write!` macro
/// using the stack instead of heap allocation. This is useful for formatting strings in the kernel
/// without allocating memory on the heap.
///
/// There are many sections of code in the kernel that are used before the heap is initialized and
/// thus cannot use the heap allocator.
pub struct BufferWriter<'a>
{
    /// The buffer to write to. It is assumed that this buffer is either staticly allocated or
    /// allocated on the stack.
    buffer: &'a mut [u8],

    /// The position of the last write to the buffer. This is used so that multiple writes to the
    /// buffer can be done without overwriting previous writes.
    position: usize
}



impl<'a> BufferWriter<'a>
{
    /// Create a new buffer writer that writes to the given buffer. The buffer must be at least 1
    /// byte long.
    pub fn new(buffer: &'a mut [u8]) -> Self
    {
        assert!(!buffer.is_empty(), "Buffer must be at least 1 byte long.");

        BufferWriter { buffer, position: 0 }
    }
}



impl<'a> Write for BufferWriter<'a>
{
    /// Write a string to the buffer. This is used to format strings using the `write!` macro.
    ///
    /// This implementation should always return `Ok(())` as we make sure that the write doesn't
    /// exceed the buffer's length. For simplicity, we don't report if the buffer gets full.
    fn write_str(&mut self, string: &str) -> fmt::Result
    {
        // Get access to the string's bytes and compute it's length, making sure that we don't write
        // past the end of the buffer.
        let bytes = string.as_bytes();
        let length = bytes.len().min(self.buffer.len() - self.position);

        // Copy the strings bytes into the buffer at the current position, then update the position
        // based on the number of bytes written.
        self.buffer[self.position..self.position + length].copy_from_slice(&bytes[..length]);
        self.position += length;

        /// TODO: If the buffer does overflow replace the last 3 characters with "..." to indicate
        ///       that the string was truncated.
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result
    {
        fmt::write(self, args)
    }
}



/// Implement the standard print! macro for printing formatted output from the kernel to an attached
/// UART device.
#[macro_export]
macro_rules! print
{
    ($($arg:tt)*) =>
        {{
            use core::{ fmt::Write, ptr::addr_of_mut };

            use crate::{ printing::{ PRINTING_UART, PRINTING_LOCK },
                         locking::LockGuard };

            unsafe
            {
                // When printing we print to the logging UART device that we initialized at boot
                // time.
                let uart = &mut *addr_of_mut!(PRINTING_UART);

                if uart.is_initialized()
                {
                    // Make sure that only one hardware thread can write to the UART at a time.
                    let _guard = LockGuard::new(&PRINTING_LOCK);

                    uart.write_fmt(format_args!($($arg)*)).unwrap();
                }
            }
        }};
}



/// Implement the standard println! macro for printing formatted output from the kernel to an
/// attached UART device. This macro appends a newline character to the end of the output.
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



/// Function to format a number as a comma-separated string. For example, 1234567 is converted to
/// the string "1,234,567".
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



/// Function to format a floating-point number as a comma-separated string, for example 1024.0 is
/// converted to the string "1,024.0".
///
/// The string is written into the provided buffer, which must be at least 64 bytes long. This
/// function will return the length of the string written to the buffer.
pub fn comma_separated_float(number: f64, buffer: &mut [u8; 64]) -> usize
{
    let mut integer_buffer = [0u8; 32];

    // Simple integer conversion
    let integer_part = number as u64;
    let fractional_part = ((number - integer_part as f64) * 10.0 + 0.5) as u64;

    // Handle case where rounding pushes us to next integer (e.g., 9.95 -> 10.0)
    let (final_integer, final_fractional) = if fractional_part >= 10
        {
            (integer_part + 1, 0)
        }
        else
        {
            (integer_part, fractional_part)
        };

    let integer_start = comma_separated_int(final_integer, &mut integer_buffer);
    let integer_len = 32 - integer_start;

    buffer[0..integer_len].copy_from_slice(&integer_buffer[integer_start..]);
    buffer[integer_len] = b'.';
    buffer[integer_len + 1] = final_fractional as u8 + b'0';

    integer_len + 2
}



/// Convert a buffer of bytes into a string.
#[macro_export]
macro_rules! buffer_as_string
{
    ($buffer:expr) =>
        {{
            use core::str;

            // SAFETY: We assume the buffer is valid UTF-8.
            unsafe { str::from_utf8_unchecked(&$buffer) }
        }};
}



/// Format a data size in a human-readable format, e.g., "1.2 MB (1,234,567 bytes)".
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



/// Initializes the printing system by finding the first UART device in the device tree and setting
/// it up for use. This function will panic if no UART device is found in the device tree.
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
