
// Bus devices are devices that provide a communication channel between the CPU and other devices.
// They are either static at runtime, like the PCI bus, or dynamic like the USB bus where devices
// can be inserted or removed at any time over the lifetime of the system.
//
// The bus device systems are responsible for providing a sub-device discovery system for drivers of
// devices that are attached to the that bus. For example, the PCI bus will discover the devices
// attached to it like a GPU or network card.
//
// Also note that VirtIO devices are treated like their own bus type since they share a common
// communication interface and are discovered in their own way in the device tree.

use alloc::collections::BTreeMap;

use xtra_kernel_shared::device_tree::DeviceTree;

use crate::devices::DeviceDriverRegistry;



/// The PCI bus subsystem responsible for discovering and managing PCI devices on the system.
pub mod pci;

/// The USB bus subsystem responsible for discovering and managing USB devices on the system.
pub mod usb;

/// VirtIO devices are treated like their own bus type since they share a common communication
/// interface.
pub mod virtio_devices;



/// The registries for the various bus types supported by the Kernel.
pub struct BusDeviceRegistry
{
    pci_drivers: BTreeMap<usize, &'static str>,
    usb_drivers: BTreeMap<usize, &'static str>
}



/// Register the bus device driver probes for bus devices, such as the PCI bus, the USB bus, etc.
pub fn register_driver_probes(registry: &mut DeviceDriverRegistry) -> Result<(), &'static str>
{
    Ok(())
}



/// Activate and initialize the bus devices discovered in the device tree. If any. We will also
/// handle PCI and USB device driver registration here as required by the busses that were actually
/// discovered in the device tree.
pub fn activate_devices() -> Result<BusDeviceRegistry, &'static str>
{
    Ok(BusDeviceRegistry
        {
            pci_drivers: BTreeMap::new(),
            usb_drivers: BTreeMap::new(),
        })
}



/// Go through the bus devices that have been discovered and run sub-device discovery and attachment
/// for the devices that are attached to each of the busses.
pub fn enumerate_bus_devices(bus_device_registry: BusDeviceRegistry) -> Result<(), &'static str>
{
    // Give ownership of the USB device driver registry to the USB bus subsystem so that it can
    // manage device attachment on demand.

    Ok(())
}
