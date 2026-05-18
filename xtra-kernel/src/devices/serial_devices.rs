
use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// Register the driver probe functions for all of the serial device drivers in the system.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the serial devices discovered in the device tree. If any.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}
