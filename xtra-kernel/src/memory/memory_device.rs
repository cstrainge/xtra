
/// The kernel's RAM and FLASH device discovery and management module.
///
/// It is important for the kernel to know what physical memory devices are attached to the running
/// hardware. This module provides the kernel with the ability to discover and manage these devices
/// so that it can properly allocate and free memory from itself and user processes.

use core::{ fmt::{ self, Display, Formatter }, str::from_utf8 };

use crate::{ device_tree::{ DeviceTree, filter_device_name } };



// TODO: Make these configurable by a global kernel configuration.
const MAX_FLASH_DEVICES: usize = 4;  /// Maximum number of flash devices we support in the system.
const MAX_RAM_DEVICES:   usize = 4;  /// Maximum number of RAM devices we support in the system.
const MAX_MMIO_REGIONS:  usize = 32; /// Maximum number of MMIO regions we support in the system.



/// It should be invalid to have a RAM device start at the end of the address space.
const INVALID_MEM_BASE_ADDRESS: usize = usize::MAX;



/// A standard FLASH device that provides the system with a contiguous block of memory that can be
/// accessed at a specific physical address and range.
///
/// It is persistent across reboots and potentially be used to store important data that needs to
/// be retained even when the system is powered off.
///
/// It can also be the storage location of boot firmware or even the kernel image itself.
///
/// Usage of this device needs to be configured per system for the device it is running on.
pub struct FlashDevice
{
    pub bank_width: u32,      // The width of the flash bank in bytes. Ie you should write this many
                              //   bytes at a time to the flash device.
    pub base_address: usize,  // The base address of the flash device in memory.
    pub range: usize,         // The range of the flash device in bytes.
}



impl FlashDevice
{
    /// Create a new instance of a FlashDevice information struct by scanning the device tree and
    /// extracting the properties of the flash device.
    ///
    /// This function will panic if the device tree does not contain the required properties or if
    /// the properties are not in the expected format.
    pub fn new(device_tree: &DeviceTree, block_offset: usize) -> Self
    {
        let mut bank_width: u32 = 0;
        let mut base_address: usize = INVALID_MEM_BASE_ADDRESS;
        let mut range: usize = 0;

        device_tree.iterate_properties(block_offset, |property_name, property_value|
            {
                match property_name
                {
                    "bank-width" =>
                        {
                            if property_value.len() != 4
                            {
                                panic!("Invalid 'bank_width' property length, \
                                       expected 4 bytes, got {} bytes.",
                                       property_value.len());
                            }

                            bank_width = u32::from_be_bytes(property_value.try_into().unwrap());
                        },

                    "reg" =>
                        {
                            if property_value.len() < 16
                            {
                                panic!("Invalid 'reg' property length, expected at least 16 bytes, \
                                       got {} bytes.", property_value.len());
                            }

                            if property_value.len() > 16
                            {
                                println!("TODO: Support multiple flash banks in the future.");
                                println!();
                            }

                            let base_bytes = property_value[0..8].try_into().unwrap();
                            let range_bytes = property_value[8..16].try_into().unwrap();

                            base_address = usize::from_be_bytes(base_bytes);
                            range = usize::from_be_bytes(range_bytes);
                        },

                    _ =>
                        {
                            // Ignore any other properties.
                        }
                }

                true
            });

        if    bank_width == 0
           || base_address == INVALID_MEM_BASE_ADDRESS
           || range == 0
        {

            panic!("Incomplete flash device properties found in the device tree.\n
                       bank_width: {}, base_address: 0x{:x}, range: {} bytes",
                   bank_width,
                   base_address,
                   range);
        }

        FlashDevice
            {
                bank_width,
                base_address,
                range
            }
    }
}



impl Display for FlashDevice
{
    /// Format the FlashDevice information for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
    {
        writeln!(f, "  FLASH Device:")?;
        writeln!(f, "    Address Range:   0x{:016x} - 0x{:016x}",
                 self.base_address,
                 self.base_address + self.range)?;
        write!(f, "    Size:            ")?;
        write_size!(f, self.range)?;
        writeln!(f)?;
        writeln!(f, "    Bank Width:      {} bytes", self.bank_width)?;

        Ok(())
    }
}



/// A standard RAM device that provides the system with a contiguous block of memory that can be
/// accessed at a specific physical address and range.
///
/// This is used by the kernel to properly manage what memory is actually available in the system
/// and to ensure that the memory allocator does not allocate pages that do not exist in the system.
pub struct MemoryDevice
{
    pub base_address: usize,  // The base mapped address of the memory device in memory.
    pub range: usize          // The range of the memory device in bytes.
}



impl MemoryDevice
{
    /// Create a new instance of a MemoryDevice information struct by scanning the device tree and
    /// extracting the properties of the memory device.
    ///
    /// This function will panic if the device tree does not contain the required properties or if
    /// the properties are not in the expected format.
    pub fn new(device_tree: &DeviceTree, block_offset: usize) -> Self
    {
        let mut base_address = INVALID_MEM_BASE_ADDRESS;  // Default to an invalid address.
        let mut range = 0;

        // Iterate through the properties of the memory device node to extract the required
        // properties.
        device_tree.iterate_properties(block_offset, |property_name, property_value|
            {
                match property_name
                {
                    "device_type" =>
                        {
                            // Convert the property value to a string and check if it is "memory".
                            // As that is the only type of RAM device we support.
                            let device_type_string = from_utf8(property_value)
                                .expect("Invalid UTF-8 in 'device_type' property.");

                            if device_type_string.trim_end_matches(|c|
                                {
                                    c == '\0' || c == ' '
                                })
                                != "memory"
                            {
                                panic!("Expected 'device_type' to be 'memory', found '{}'.",
                                device_type_string);
                            }
                        },

                    "reg" =>
                        {
                            // Is the property the correct size?
                            if property_value.len() < 16
                            {
                                panic!("Invalid 'reg' property length, expected at least 16 bytes, \
                                       got {} bytes.", property_value.len());
                            }

                            // The 'reg' property is expected to be a pair of 8-byte values: base
                            // address and range.
                            let base_bytes = property_value[0..8].try_into().unwrap();
                            let range_bytes = property_value[8..16].try_into().unwrap();

                            base_address = usize::from_be_bytes(base_bytes);
                            range = usize::from_be_bytes(range_bytes);
                        },

                    _ =>
                        {
                            // Ignore any other properties. In the current spec this code is written
                            // for there shouldn't be any other properties.
                            //
                            // But to future proof things, we don't panic if we discover them.
                        }
                }

                true
            });

        // Make sure that the required properties were found and are valid. We can't really check
        // base address for zero because some systems may have a memory device that starts at
        // address zero, so we check to see if the base address is the invalid address we set
        // above.
        //
        // We also make sure that the range is not zero, because that would mean the device isn't
        // actually usable.
        if    base_address == INVALID_MEM_BASE_ADDRESS
           || range == 0
        {
            panic!("Incomplete memory device properties found in the device tree.");
        }

        MemoryDevice
            {
                base_address,
                range
            }
    }
}



impl Display for MemoryDevice
{
    /// Format the MemoryDevice information for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
    {
        writeln!(f, "  RAM Device:")?;
        writeln!(f, "    Address Range:   0x{:016x} - 0x{:016x}",
                 self.base_address,
                 self.base_address + self.range)?;
        write!(f, "    Size:            ")?;
        write_size!(f, self.range)?;
        writeln!(f)?;

        Ok(())
    }
}



/// The kernel needs to know where memory mapped I/O devices are mapped in the device's physical
/// address space. This way we can properly map them into the kernel's virtual address space
/// and access them as needed.
pub struct MmioRegion
{
    pub base_address: usize,  // The base address of the MMIO region in memory.
    pub range: usize          // The range of the MMIO region in bytes.
}



impl MmioRegion
{
    /// Create a new instance of a MMIO region from a start and end address.
    pub fn from_range(base_address: usize, range: usize) -> Self
    {
        MmioRegion
            {
                base_address,
                range
            }
    }
}



impl Display for MmioRegion
{
    /// Format the MMIO region information for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
    {
        writeln!(f, "  MMIO Region:")?;
        writeln!(f, "    Address Range:   0x{:016x} - 0x{:016x}",
                 self.base_address,
                 self.base_address + self.range)?;
        write!(f, "    Size:            ")?;
        write_size!(f, self.range)?;
        writeln!(f)?;

        Ok(())
    }
}



/// Information about the memory devices found in the system at boot time. Some systems my have
/// multiple FLASH and RAM devices mapped to different address regions in the system.  The memory
/// allocator will need this to make sure it doesn't dole out a memory page that doesn't exist.
///
/// We can also in the future provide a special FLASH device in the system that can be used to
/// read and write from the found FLASH device(s).
pub struct SystemMemory
{
    pub flash_devices: [Option<FlashDevice>; MAX_FLASH_DEVICES],  // The FLASH device(s).
    pub memory_devices: [Option<MemoryDevice>; MAX_RAM_DEVICES],  // The RAM device(s).
    pub mmio_regions: [Option<MmioRegion>; MAX_MMIO_REGIONS]      // The MMIO region(s).
}



impl SystemMemory
{
    /// Create a new `SystemMemory` instance by scanning the device tree and looking for a memory
    /// device node.
    ///
    /// This function will panic if no memory device node is found in the device tree.
    pub fn new(device_tree: &DeviceTree) -> Self
    {
        // Start off assuming we found nothing.
        let mut memory_devices: [Option<MemoryDevice>; MAX_RAM_DEVICES] = Default::default();
        let mut flash_devices: [Option<FlashDevice>; MAX_FLASH_DEVICES] = Default::default();
        let mut mmio_regions: [Option<MmioRegion>; MAX_MMIO_REGIONS] = Default::default();

        let mut memory_devices_found = 0;
        let mut flash_devices_found = 0;
        let mut mmio_regions_found = 0;

        // Iterate through the device tree to find the RAM and FLASH device node(s).
        device_tree.iterate_blocks(|block_offset, device_name|
            {
                let device_name = filter_device_name(device_name);

                match device_name
                {
                    "memory" =>
                        {
                            // Make sure we're in range of the maximum number of RAM devices.
                            if memory_devices_found >= MAX_RAM_DEVICES
                            {
                                panic!("Too many RAM devices found in the device tree, \
                                       maximum supported is {}.", MAX_RAM_DEVICES);
                            }

                            // Add the RAM device to the list of memory devices.
                            memory_devices[memory_devices_found]
                                = Some(MemoryDevice::new(device_tree, block_offset));

                            memory_devices_found += 1;
                        },

                    "flash" =>
                        {
                            // Make sure we're in range of the maximum number of flash devices.
                            if flash_devices_found >= MAX_FLASH_DEVICES
                            {
                                panic!("Too many FLASH devices found in the device tree, \
                                       maximum supported is {}.", MAX_FLASH_DEVICES);
                            }

                            // Add the FLASH device to the list of flash devices.
                            flash_devices[flash_devices_found]
                                = Some(FlashDevice::new(device_tree, block_offset));

                            flash_devices_found += 1;
                        },

                    _ =>
                        {
                            // For all other device types we check to see if it has a 'reg' property
                            // and if so, we assume it is a MMIO device and add it to the MMIO
                            // region.
                            let found = Self::get_mmio_device_range(device_tree, block_offset);

                            // Check to see if we found a valid MMIO device range.
                            if let Some((start, range)) = found
                            {
                                // Make sure we're in range of the maximum number of MMIO regions.
                                if mmio_regions_found >= MAX_MMIO_REGIONS
                                {
                                    panic!("Too many MMIO regions found in the device tree, \
                                           maximum supported is {}.", MAX_MMIO_REGIONS);
                                }

                                // Add the MMIO region to the list of MMIO regions.
                                mmio_regions[mmio_regions_found]
                                    = Some(MmioRegion::from_range(start, range));

                                mmio_regions_found += 1;
                            }
                        }
                }

                true
            });

        // Did we find any RAM devices in the device tree? If not something is messed up with the
        // system configuration because obviously the device is running with _some_ RAM.
        if memory_devices_found == 0
        {
            panic!("No memory device found in the device tree.");
        }

        SystemMemory
            {
                flash_devices,
                memory_devices,
                mmio_regions
            }
    }

    /// Look to see if the given device tree block has a reg property, if it does than that means it
    /// is a MMIO device and we can extract it's range.
    ///
    /// Otherwise we return None.
    fn get_mmio_device_range(device_tree: &DeviceTree,
                             block_offset: usize) -> Option<(usize, usize)>
    {
        let mut base_address = INVALID_MEM_BASE_ADDRESS;
        let mut range = 0;

        // Iterate through the properties of the given device block and extract a reg property if it
        // has one.
        device_tree.iterate_properties(block_offset, |property_name, property_value|
            {
                if property_name == "reg"
                {
                    // Is the property the correct size?
                    if property_value.len() == 16
                    {
                        // The 'reg' property is expected to be a pair of 8-byte values: base
                        // address and range.
                        let base_bytes = property_value[0..8].try_into().unwrap();
                        let range_bytes = property_value[8..16].try_into().unwrap();

                        base_address = usize::from_be_bytes(base_bytes);
                        range = usize::from_be_bytes(range_bytes);
                    }

                    // We found a valid MMIO reg property, so we don't need to iterate any further.
                    false
                }
                else
                {
                    true
                }
            });

        // Ok, check if we found a valid MMIO device range.
        if    base_address != INVALID_MEM_BASE_ADDRESS
           && range != 0
        {
            // We found a valid MMIO device range.
            Some((base_address, range))
        }
        else
        {
            // No valid MMIO device range found.
            None
        }
    }
}



impl Display for SystemMemory
{
    /// Format the SystemMemory information for display purposes.
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error>
    {
        writeln!(f, "System Memory Information:")?;

        if self.flash_devices.iter().any(|d| d.is_some())
        {
            for device in self.flash_devices.iter()
            {
                if let Some(device) = device
                {
                    write!(f, "{}", device)?;
                }
            }
        }
        else
        {
            writeln!(f, "  No FLASH devices found.")?;
        }

        if self.memory_devices.iter().any(|d| d.is_some())
        {
            for device in self.memory_devices.iter()
            {
                if let Some(device) = device
                {
                    write!(f, "{}", device)?;
                }
            }
        }
        else
        {
            writeln!(f, "  No RAM devices found.")?;
        }

        if self.mmio_regions.iter().any(|d| d.is_some())
        {
            for region in self.mmio_regions.iter()
            {
                if let Some(region) = region
                {
                    write!(f, "{}", region)?;
                }
            }
        }
        else
        {
            writeln!(f, "  No MMIO regions found.")?;
        }

        Ok(())
    }
}
