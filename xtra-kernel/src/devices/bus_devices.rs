
use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// Register the bus device driver probes for bus devices, such as the PCI bus, the USB bus, etc.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}


/// Activate and initialize the bus devices discovered in the device tree. If any.
pub fn activate_devices() -> Result<(), &'static str>
{
    Ok(())
}
