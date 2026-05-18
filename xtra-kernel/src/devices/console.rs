
/// Initialize the virtual console devices. We will search for devices that can be used as virtual
/// consoles and initialize them so that they can be used by the rest of the kernel for console
/// I/O.
///
/// For example we will take charge of a graphics device and use it as a framebuffer console, or we
/// will take charge of serial ports and use them as serial consoles.
///
/// Later phases of the boot can disconnect the console from the underlying hardware and use it for
/// other purposes. At which point that virtual console will be freed and will be no longer
/// available for use by the kernel or user processes.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}
