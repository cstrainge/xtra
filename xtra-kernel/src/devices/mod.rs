
// Heart of the device driver subsystem. This module contains the code for walking the device tree,
// matching device tree blocks with device drivers, and initializing the devices. It also contains
// physical device drivers and virtual devices drivers that sit on top of the physical devices,
// such as console device drivers.

use alloc::{ format, collections::BTreeMap};

use xtra_kernel_shared::device_tree::{ DeviceTree };



/// Bus devices on the machine, such as the PCI bus, the USB bus, etc.
pub mod bus_devices;

/// Block devices, such as hard drives, SSDs, etc.
pub mod block_devices;

///
pub mod clint;

/// Console devices, such as the VGA console, the serial console, etc.
pub mod console;

/// CPU devices, such as the CPU cores, the CPU cache, etc.
pub mod cpu_devices;

/// Graphics devices, such as GPUs, display controllers, etc.
pub mod graphics_devices;

/// Human Interface Devices (HID), such as keyboards, mice, touch pads, etc they tend to depend on
/// an external bus like USB or PS/2.
pub mod hid_devices;

/// Interrupt controllers, such as the PLIC, the APIC, etc.
pub mod interrupt_controllers;

/// Memory mapped I/O control interfaces for devices.
pub mod mmio_devices;

/// Network devices, such as Ethernet controllers, Wi-Fi adapters, etc.
pub mod network_devices;

///
pub mod plic_devices;

/// System level power control devices, such as the shutdown or restart controllers.
pub mod power_devices;

/// Serial devices, such as the UART, the 16550, etc.
pub mod serial_devices;

///
pub mod test_devices;

/// Timer devices, such as the system timer, the RTC, etc.
pub mod timer_devices;

/// Virtio device interface handler code. This is shared between the various virtio device types,
/// such as block, network, graphics etc.
pub mod virtio_devices;




/// Type for the function that gets called to probe the device tree and create the data structures
/// the driver will need to manage the device.
///
/// The function will be called with the device tree name and the address of the device if it is
/// specified in the name. The device tree object is supplied so that the driver can perform device
/// specific parsing of the device tree block.
pub type DriverProbeFunction = fn(name: &str,
                                  address: Option<usize>,
                                  device_tree: &DeviceTree) -> Result<(), &'static str>;


/// The device driver registry type, this is a mapping from device tree node names to the driver
/// probe functions that can handle those nodes.
///
/// When the device tree is walked it will be these registered handler functions that will be called
/// to do the raw block specific parsing and device structure allocation for the devices represented
/// by the device tree blocks.
pub type DeviceDriverRegistry = BTreeMap<&'static str, DriverProbeFunction>;



/// Initialize the device registry, this will set up the data structures for storing the device
/// driver to device tree block mappings.
pub fn initialize_device_registry() -> Result<DeviceDriverRegistry, &'static str>
{
    let mut registry = DeviceDriverRegistry::new();

    // Go through all of the device driver subsystems and register their driver probe functions in
    // the device registry. The order of registration here does not matter.
    //
    // Note that we skip virtual devices like the console device drivers here because they are not
    // directly tied to any hardware and thus don't have a probe function to call.
    block_devices::register_driver_probes(&mut registry)?;
    bus_devices::register_driver_probes(&mut registry)?;
    clint::register_driver_probes(&mut registry)?;
    cpu_devices::register_driver_probes(&mut registry)?;
    hid_devices::register_driver_probes(&mut registry)?;
    graphics_devices::register_driver_probes(&mut registry)?;
    interrupt_controllers::register_driver_probes(&mut registry)?;
    mmio_devices::register_driver_probes(&mut registry)?;
    network_devices::register_driver_probes(&mut registry)?;
    plic_devices::register_driver_probes(&mut registry)?;
    power_devices::register_driver_probes(&mut registry)?;
    serial_devices::register_driver_probes(&mut registry)?;
    test_devices::register_driver_probes(&mut registry)?;
    timer_devices::register_driver_probes(&mut registry)?;
    virtio_devices::register_driver_probes(&mut registry)?;

    Ok(registry)
}


/// Walk the device tree and construct the device drivers for the hardware actually represented in
/// the machine.
pub fn walk_device_tree(device_tree: &DeviceTree,
                        device_registry: DeviceDriverRegistry) -> Result<(), &'static str>
{
    // Walk the device tree and initialize the devices based on the device tree blocks and the
    // registered device drivers in the device registry.
    device_tree.iterate_blocks(|tree_offset, raw_node_name|
        {
            // Split the node name from the address if there's an @ symbol in the node name.
            let (name, address) = if let Some(at_index) = raw_node_name.find('@')
                {
                    let name = &raw_node_name[..at_index];
                    let address_str = &raw_node_name[at_index + 1..];

                    // Try to parse the address as a hexadecimal number.
                    let address = usize::from_str_radix(address_str, 16)
                        .map_err(|error|
                            {
                                format!("Failed to parse device tree node address: {}", error)
                            });

                    if let Err(error) = address
                    {
                        println!("Failed to parse address for device tree node {}: {}, skipping \
                                  device",
                                  raw_node_name,
                                  error);

                        /// Ok, this block seems broken, but skip onto the next block.
                        return true;
                    }

                    (name, Some(address.unwrap()))
                }
                else
                {
                    (raw_node_name, None)
                };

            // Check if we have a device driver registered for this node name. If we do, call the
            // probe function to initialize the device.
            if let Some(probe_function) = device_registry.get::<str>(name)
            {
                let result = probe_function(name, address, device_tree)
                    .map_err(|err|
                        {
                            format!("Failed to initialize device driver for node {}: {}", name, err)
                        });

                if let Err(error) = result
                {
                     println!("Failed to initialize device driver for node {}: {}, skipping device",
                              name,
                              error);
                }
            }

            // Keep iterating through the device tree blocks.
            true
        });

    Ok(())
}


/// Activate all of the discovered devices from the device tree. This will connect the devices to
/// their hardware specific drivers and make them available for use by the rest of the kernel.
pub fn activate_devices() -> Result<(), &'static str>
{
    // Activate and initialize the devices that we have discovered. We have to be careful here
    // because many devices have dependencies on other devices. For example most devcies depend on
    // the interrupt controller initialized and active so that they can register their interrupt
    // handlers.
    cpu_devices::activate_devices()?;
    interrupt_controllers::activate_devices()?;
    virtio_devices::activate_devices()?;
    mmio_devices::activate_devices()?;
    bus_devices::activate_devices()?;
    power_devices::activate_devices()?;
    timer_devices::activate_devices()?;
    clint::activate_devices()?;
    plic_devices::activate_devices()?;
    block_devices::activate_devices()?;
    serial_devices::activate_devices()?;
    test_devices::activate_devices()?;
    graphics_devices::activate_devices()?;
    hid_devices::activate_devices()?;

    // Activate the virtual devices that sit on top of the physical devices, such as the console
    // device drivers.
    console::activate_devices()?;

    Ok(())
}
