
#include <stdint.h>



// Base address for UART0
constexpr uint32_t UART0_BASE = 0x10000000;

// Transmit Holding Register.
volatile uint8_t* uart0_thr = reinterpret_cast<volatile uint8_t*>(UART0_BASE + 0x00);

// Line Status Register.
volatile uint8_t* uart0_lsr = reinterpret_cast<volatile uint8_t*>(UART0_BASE + 0x5);


static void uart0_init()
{
    // Set the baud rate, data bits, stop bits, and parity.
    // This is typically done by configuring the UART registers.
    // For simplicity, we assume the UART is already configured correctly.
}


static void uart0_putc(char c)
{
    // Wait for the Transmit Holding Register to be empty.
    while ((*uart0_lsr & 0x20) == 0) { /* spin */ }

    // Write the character to the Transmit Holding Register.
    *uart0_thr = c;
}


static void uart0_write_string(const char* str)
{
    while (*str)
    {
        uart0_putc(*str);
        str++;
    }
}


static void uart0_write_hex(uint64_t value)
{
    const char* hex_digits = "0123456789abcdef";

    for (int i = 15; i >= 0; --i)
    {
        uart0_putc(hex_digits[(value >> (i * 4)) & 0xF]);
    }
}


static void power_off_qemu()
{
    uart0_write_string("xtra-os kernel is powering off QEMU.\n");

    // Write a specific value to the power control register to power off QEMU.
    volatile int* power_ctrl = (volatile int*)0x100000;

    *power_ctrl = 0x5555;  // The command to power off QEMU.

    while (1)
    {
        asm volatile("wfi");  // Wait for interrupt
    }
}


extern "C" void main(uint64_t heart_id, uintptr_t dtb_address)
{
    uart0_init();

    uart0_write_string("xtra-os kernel started.\n");
    uart0_write_string("Heart ID: ");
    uart0_write_hex(heart_id);
    uart0_write_string("\n");

    uart0_write_string("\nDevice Tree Blob Address: ");
    uart0_write_hex(dtb_address);
    uart0_write_string("\n");

    power_off_qemu();
}
