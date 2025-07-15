
// Implementation of a simple VirtIO MMIO UART for logging output in a no_std environment. This
// version doesn't support interrupts or reading from the UART, it is really just intended for
// simple logging output from the kernel to an attached device.

use core::{ fmt::{ self, Write }, hint::spin_loop, ptr::{ read_volatile, write_volatile } };



// Indices of the UART MMIO registers.
const UART_THR: usize = 0; // Transmit Holding Register.
const UART_IER: usize = 1; // Interrupt Enable Register.
const UART_LCR: usize = 3; // Line Control Register.
const UART_LSR: usize = 5; // Line Status Register.



// Implementation of a UART that doesn't use interrupts for communication. It also doesn't support
// reading from the UART. This is intended for simple logging output from the Kernel to an attached
// device.
//
// Or from a virtual machine to the host console like QEMU.
pub struct SimpleUart
{
    base: usize
}


impl SimpleUart
{
    // Create a new UART device with the specified base address, but leave it uninitialized. This
    // method is useful when needing to access an already initialized UART but you don't have a
    // reference to the main UART instance.
    pub const fn new(base: usize) -> SimpleUart
    {
        SimpleUart { base }
    }

    // Initialize the UART with the specified base address but also set it up for use.
    pub fn init_new(base: usize) -> SimpleUart
    {
        let uart = SimpleUart::new(base);

        uart.init();

        uart
    }

    /// Initializes the UART for use. This sets up the line control register and disables
    /// interrupts.
    pub fn init(&self)
    {
        // Set the Line Control Register to 8 bits, no parity, 1 stop bit.
        self.set_lcr(0b_0000_0011);

        // Disable the UART interrupts.
        self.set_ier(0b_0000_0000);
    }

    /// Creates a new Uart instance with a base address of 0, that is non-functional.
    pub const fn zeroed() -> SimpleUart
    {
        SimpleUart { base: 0 }
    }

    // Is the UART initialized?
    pub fn is_initialized(&self) -> bool
    {
        // If the base address is 0, then the UART is not initialized.
        self.base != 0
    }

    // Write a character to the UART's output buffer. If the buffer is full we will busy wait until
    // it is ready to accept more data.
    pub fn put_char(&self, c: u8)
    {
        // Wait for the Transmit Holding Register to be empty.
        while (self.get_lsr() & 0b_0010_0000) == 0
        {
            // Play the waiting game, but let the compiler know this is a busy wait.
            spin_loop();
        }

        // Write the character to the Transmit Holding Register.
        self.set_thr(c);
    }

    // Write a string to the UART's output buffer. This will convert newlines to carriage return +
    // and new-line characters for better formatting on the console.
    pub fn put_str(&self, s: &str)
    {
        // Simply iterate over the string and write each character to the UART filtering \n
        // characters as we go.
        for c in s.bytes()
        {
            if c == b'\n'
            {
                // Convert newline to carriage return + newline.
                self.put_char(b'\r');
            }

            // Write the character to the UART.
            self.put_char(c);
        }
    }

    // Write to the UART's Line Control Register (LCR).
    fn set_lcr(&self, lcr: u8)
    {
        unsafe
        {
            write_volatile((self.base + UART_LCR) as *mut u8, lcr);
        }
    }

    // Write to the UART's Interrupt Enable Register (IER) to disable or enable interrupts.
    fn set_ier(&self, ier: u8)
    {
        unsafe
        {
            write_volatile((self.base + UART_IER) as *mut u8, ier);
        }
    }

    // Read the Line Status Register (LSR) to check if the UART is ready to accept more data.
    fn get_lsr(&self) -> u8
    {
        unsafe
        {
            read_volatile((self.base + UART_LSR) as *const u8)
        }
    }

    // Write a byte to the Transmit Holding Register (THR) to send data to the connected device.
    fn set_thr(&self, thr: u8)
    {
        unsafe
        {
            write_volatile((self.base + UART_THR) as *mut u8, thr);
        }
    }
}



impl Write for SimpleUart
{
    fn write_str(&mut self, s: &str) -> fmt::Result
    {
        // Write the string to the UART, converting newlines to carriage return + newline.
        self.put_str(s);
        Ok(())
    }
}
