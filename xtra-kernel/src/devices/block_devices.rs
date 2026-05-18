
use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// Register the device driver probes for block devices, such as hard drives, SSDs, etc.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the block devices discovered in the device tree. If any.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}
