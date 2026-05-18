
use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// Register the driver probe functions for all of the CPUs in the system.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the CPU information devices discovered in the device tree. It would be
/// pretty weird if we didn't find any CPUs in the system.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}
