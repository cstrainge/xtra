
use core::{ arch::asm, ptr::write_volatile };



// Address of the power control register in QEMU.
const POWER_CONTROL_REGISTER_ADDRESS: usize = 0x0010_0000;

// The command to power off the system.
const POWER_OFF_COMMAND: u32 = 0x0000_5555;

// The command to reset the system.
const RESET_COMMAND: u32 = 0x0000_7777;



// Trigger a system power off. This function will not return.
pub fn power_off() -> !
{
    let power_control_ptr = POWER_CONTROL_REGISTER_ADDRESS as *mut u32;

    // Write the reset command to the power control register making sure that the write is volatile
    // so that the compiler does not optimize it away.
    unsafe
    {
        write_volatile(power_control_ptr, POWER_OFF_COMMAND);
        wait_for_interrupt();
    }
}


// Trigger a system reset. This function will not return.
pub fn reset() -> !
{
    let power_control_ptr = POWER_CONTROL_REGISTER_ADDRESS as *mut u32;

    // Write the reset command to the power control register making sure that the write is volatile
    // so that the compiler does not optimize it away.
    unsafe
    {
        write_volatile(power_control_ptr, RESET_COMMAND);
        wait_for_interrupt();
    }
}


// Just loop forever, waiting for an interrupt.
pub unsafe fn wait_for_interrupt() -> !
{
    loop
    {
        asm!
        (
            "wfi",
            options(nomem, nostack, preserves_flags)
        );
    }
}
