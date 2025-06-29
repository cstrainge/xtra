
#include <stdint.h>


// Base address for UART0.
constexpr uintptr_t uart0_base = 0x10000000;

// Transmit Holding Register.
volatile uint8_t* uart0_thr = reinterpret_cast<volatile uint8_t*>(uart0_base + 0x00);

// Line Status Register.
volatile uint8_t* uart0_lsr = reinterpret_cast<volatile uint8_t*>(uart0_base + 0x5);



// Write a character to the UART0 Transmit Holding Register, we'll also make sure that we don't
// write to the register until it is ready to accept a new character.
static void write_char(char c)
{
    // Wait for the Transmit Holding Register to be empty.
    while ((*uart0_lsr & 0x20) == 0) { /* spin */ }

    // Write the character to the Transmit Holding Register.
    *uart0_thr = c;
}


// Write a string to the first UART device.
static void write_string(const char* str)
{
    while (*str)
    {
        write_char(*str);
        str++;
    }
}


// Write a hexadecimal value to the first UART device.
static void write_hex(uint64_t value)
{
    const char* hex_digits = "0123456789abcdef";

    for (int i = 15; i >= 0; --i)
    {
        write_char(hex_digits[(value >> (i * 4)) & 0xF]);
    }
}


// Power off the QEMU virtual machine.
static void power_off_qemu()
{
    // Write a specific value to the power control register to power off QEMU.
    volatile int* power_control_register = (volatile int*)0x100000;

    *power_control_register = 0x5555;  // The command to power off QEMU.

    // Just wait for interrupts until the power off command is executed.
    while (1)
    {
        asm volatile("wfi");  // Wait for interrupt
    }
}


extern "C" void main(uint64_t heart_id, uintptr_t dtb_address)
{
    // Make sure that we are running on the primary heart.
    if (heart_id != 0)
    {
        // This is a secondary heart, so we just halt and wait for the primary heart to take over.
        while (1)
        {
            asm volatile("wfi");  // Wait for interrupt.
        }
    }

    // Announce ourself to the world.
    write_string("xtra-os boot-loader started.\n");
    write_string("Heart ID: ");
    write_hex(heart_id);
    write_string("\n");

    write_string("\nDevice Tree Blob Address: ");
    write_hex(dtb_address);
    write_string("\n");

    // Power off the QEMU virtual machine.
    power_off_qemu();
}
