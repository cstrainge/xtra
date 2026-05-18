
use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// Register the driver probe functions for all of the interrupt controllers in the system.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the interrupt controllers discovered in the device tree. If any. Though,
/// it would be a highly unusual system if we didn't find any interrupt controllers in the device
/// tree.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}
