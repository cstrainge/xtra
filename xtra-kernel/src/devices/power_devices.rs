
use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// Register the driver probe functions for all of the block device drivers in the system.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the block devices discovered in the device tree. If any.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}


/// Power down the system. This should never return if it succeeds.
pub fn power_down_system() -> Result<!, &'static str>
{
    Err("System power down has not been implemented.")
}


/// Temporarily suspend the system.
pub fn suspend_system() -> Result<(), &'static str>
{
    Err("System suspend has not been implemented.")
}


/// Reboot the system. This should never return if it succeeds.
pub fn reboot_system() -> Result<!, &'static str>
{
    Err("System reboot has not been implemented.")
}
