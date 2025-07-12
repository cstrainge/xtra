
use core::ptr::{ read_volatile, write_volatile };



pub const UART_0_BASE: usize = 0x1000_0000;

const UART_THR: usize = 0; // Transmit Holding Register.
//const UART_RBR: usize = 0; // Receive Buffer Register  .
const UART_IER: usize = 1; // Interrupt Enable Register.
const UART_LCR: usize = 3; // Line Control Register.
const UART_LSR: usize = 5; // Line Status Register.



pub struct Uart
{
    base: usize
}


impl Uart
{
    pub const fn new(base: usize) -> Uart
    {
        Uart { base }
    }

    pub fn init_new(base: usize) -> Uart
    {
        let uart = Uart::new(base);

        uart.init();

        uart
    }

    pub fn init(&self)
    {
        // Set the Line Control Register to 8 bits, no parity, 1 stop bit.
        self.set_lcr(0b_0000_0011);

        // Disable the UART interrupts.
        self.set_ier(0b_0000_0000);
    }

    pub fn put_char(&self, c: u8)
    {
        // Wait for the Transmit Holding Register to be empty.
        while (self.get_lsr() & 0b_0010_0000) == 0
        {
            // Play the waiting game.
        }

        // Write the character to the Transmit Holding Register.
        self.set_thr(c);
    }

    pub fn put_str(&self, s: &str)
    {
        for c in s.bytes()
        {
            if c == b'\n'
            {
                // Convert newline to carriage return + newline.
                self.put_char(b'\r');
            }

            self.put_char(c);
        }
    }

    pub fn put_int(&self, n: usize)
    {
        if n == 0
        {
            self.put_char(b'0');
            return;
        }

        let mut num = n;
        let mut digits = [0u8; 20];
        let mut i = 0;

        while num > 0
        {
            digits[i] = (num % 10) as u8 + b'0';
            num /= 10;
            i += 1;
        }

        // Print the digits in reverse order.
        for j in (0..i).rev()
        {
            self.put_char(digits[j]);
        }
    }

    pub fn put_hex(&self, n: usize, prefix: bool)
    {
        if prefix
        {
            self.put_char(b'0');
            self.put_char(b'x');
        }

        if n == 0
        {
            self.put_char(b'0');
            return;
        }

        let mut num = n;
        let mut digits = [0u8; 16];
        let mut i = 0;

        while num > 0
        {
            let digit = (num & 0xF) as u8;
            digits[i] = if digit < 10 { digit + b'0' } else { digit - 10 + b'a' };
            num >>= 4;
            i += 1;
        }

        // Print the digits in reverse order.
        for j in (0..i).rev()
        {
            self.put_char(digits[j]);
        }
    }

    // Print a byte array as hex values, separated by spaces.
    pub fn put_hex_bytes(&self, bytes: &[u8], max_bytes: Option<usize>)
    {
        for (i, &byte) in bytes.iter().enumerate()
        {
            if byte < 0x10
            {
                // Pad single hex digits with a leading zero.
                self.put_char(b'0');
            }

            self.put_hex(byte as usize, false);

            if    let Some(max_bytes) = max_bytes
               && i + 1 >= max_bytes
               && i + 1 < bytes.len()
            {
                self.put_str("...");
                break;
            }

            if i < bytes.len() - 1
            {
                self.put_char(b' ');
            }
        }
    }

    pub fn put_hex_byte(&self, byte: u8)
    {
        // See if we need to pad the byte with a leading zero.
        if byte < 0x10
        {
            // Pad single hex digits with a leading zero.
            self.put_char(b'0');
        }

        // Print the byte as hex.
        self.put_hex(byte as usize, false);
    }

    pub fn put_hex_address(&self, address: usize)
    {
        // Pad the address with leading zeros to match the specified byte size.
        let hex_length = 8;
        let mut hex_chars = [b'0'; 8];

        for i in (0..hex_length).rev()
        {
            let shift = (hex_length - 1 - i) * 4;
            let digit = ((address >> shift) & 0xF) as u8;
            let hex_char = if digit < 10 { digit + b'0' } else { digit - 10 + b'a' };

            hex_chars[i] = hex_char;
        }

        // Print the hex address with leading zeros.
        for i in 0..hex_length
        {
            self.put_char(hex_chars[i]);
        }
    }

    pub fn put_hex_dump(&self, bytes: &[u8])
    {
        self.put_str("          ");
        self.put_str("00 01 02 03 04 05 06 07  08 09 0a 0b 0c 0d 0e 0f  | 01234567 89abcdef |\n");

        for (chunk_index, chunk) in bytes.chunks(16).enumerate()
        {
            let offset = chunk_index * 16;

            self.put_hex_address(offset);
            self.put_str("  ");

            for index in 0..16
            {
                if index == 8
                {
                    // Add a space after the 8th byte for formatting.
                    self.put_char(b' ');
                }

                if index < chunk.len()
                {
                    // Print the byte as hex.
                    self.put_hex_byte(chunk[index]);
                    self.put_char(b' ');
                }
                else
                {
                    // Print a space for missing bytes.
                    self.put_char(b' ');
                    self.put_char(b' ');
                }
            }

            self.put_str(" | ");

            for (index, &byte) in chunk.iter().enumerate()
            {
                if index == 8
                {
                    // Add a space after the 8th byte for formatting.
                    self.put_char(b' ');
                }

                if    byte.is_ascii_alphanumeric()
                   || byte.is_ascii_punctuation()
                   || byte == b' '
                {
                    // Print the byte as a character if it's printable.
                    self.put_char(byte);
                }
                else
                {
                    // Print a dot for non-printable characters.
                    self.put_char(b'.');
                }
            }

            for index in chunk.len()..16
            {
                if index == 8
                {
                    // Add a space after the 8th byte for formatting.
                    self.put_char(b' ');
                }

                // Print a dot for missing bytes.
                self.put_char(b'.');
            }

            self.put_str(" |\n");
        }
    }

    fn set_lcr(&self, lcr: u8)
    {
        unsafe
        {
            write_volatile((self.base + UART_LCR) as *mut u8, lcr);
        }
    }

    fn set_ier(&self, ier: u8)
    {
        unsafe
        {
            write_volatile((self.base + UART_IER) as *mut u8, ier);
        }
    }

    fn get_lsr(&self) -> u8
    {
        unsafe
        {
            read_volatile((self.base + UART_LSR) as *const u8)
        }
    }

    fn set_thr(&self, thr: u8)
    {
        unsafe
        {
            write_volatile((self.base + UART_THR) as *mut u8, thr);
        }
    }
}
