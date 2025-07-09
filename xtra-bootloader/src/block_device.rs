
use core::{ mem::size_of, ptr::{ read_volatile, write_volatile }, str, time };

use crate::{ device_tree::DeviceTree,
             partition_table::{ MasterBootRecord, Partition },
             uart::Uart,
             virtio::VirtIoBlockDevice };



// Represents the range of registers for a block device.
#[derive(Clone, Copy)]
struct Registers
{
    pub base: usize,
    pub size: usize
}



impl Registers
{
    // Creates a new Registers instance with the specified base address and size.
    pub fn new(base: usize, size: usize) -> Self
    {
        Registers { base, size }
    }
}



const SECTOR_SIZE: usize = 512;        // Standard sector size for block devices.



// Representation of a storage device in the system.
//
// This could be a hard drive, SSD, or any other block device that can be used to store data. The
// bootloader will use this to find the kernel image and load it.
pub struct BlockDevice
{
    registers: Registers,            // The register set for the block device.
    interrupts: u32,                 // Interrupts for the device.
    interrupt_parent: u32,           // Parent interrupt controller.

    virt_device: VirtIoBlockDevice,  // The VirtIO block device driver that provides the interface
                                     // to the block device.

    mbr: Option<MasterBootRecord>    // The MBR for the block device loaded from the first sector of
                                     // the device.
}



impl BlockDevice
{
    fn new(registers: Registers, interrupts: u32, interrupt_parent: u32) -> Self
    {
        BlockDevice
            {
                registers: registers.clone(),
                interrupts,
                interrupt_parent,

                virt_device: VirtIoBlockDevice::new(registers.base),
                mbr: None
            }
    }


    pub fn find_first_drive(uart: &Uart, device_tree: DeviceTree) -> Option<BlockDevice>
    {
        let mut block_device = None;

        // Iterate though the device tree and try to find a suitable block device for booting from.
        device_tree.iterate_blocks(|offset, name|
            {
                // Look for the @ and extract the device name as a substring. If there isn't an @
                // then we assume the whole name is the device name.
                let device_name = if let Some(at_index) = name.find('@')
                    {
                        &name[..at_index]
                    }
                    else
                    {
                        name
                    };

                // For now assume we're looking for a VirtIO block device, so we'll check for
                // the "virtio,mmio" compatible string.
                if device_name == "virtio_mmio"
                {
                    let mut interrupts: u32 = 0;
                    let mut interrupt_parent: u32 = 0;
                    let mut registers: Registers = Registers::new(0, 0);
                    let mut compatible = false;

                    // We found a virtio device, so let's probe it for more information. Start off
                    // by iterating the listed device properties.
                    device_tree.iterate_properties(offset, |prop_name, prop_value|
                        {
                            match prop_name
                            {
                                "interrupts" =>
                                    {
                                        interrupts = Self::property_to_u32(prop_value);
                                    },

                                "interrupt-parent" =>
                                    {
                                        interrupt_parent = Self::property_to_u32(prop_value);
                                    },

                                "reg" =>
                                    {
                                        // We're expecting the 'reg' property to be a 16-byte, two
                                        // 64-bit values.
                                        if prop_value.len() != 16
                                        {
                                            panic!("Invalid 'reg' property length.");
                                        }

                                        // Extract the integers from the byte array.
                                        registers.base = Self::property_to_u64(&prop_value[0..8]);
                                        registers.size = Self::property_to_u64(&prop_value[8..16]);
                                    },

                                "compatible" =>
                                    {
                                        compatible = Self::is_compatible(prop_value, "virtio,mmio");
                                    },

                                _ =>
                                    {
                                        // Ignore any other properties for now.
                                    }
                            }

                            true
                        });

                    // Check to see if we have found a valid VirtIO block device.
                    if compatible
                    {
                        // Now make sure that the device looks useable.
                        let mut device = BlockDevice::new(registers, interrupts, interrupt_parent);

                        if device.virt_device.is_block_device() == true
                        {
                            block_device = Some(device);

                            return false;
                        }
                    }
                }

                true
            });

        block_device
    }

    // Initialize the block device for communication.
    pub fn initialize(&mut self, uart: &Uart)
    {
        uart.put_str("Initializing block device...\n");

        let result = self.virt_device.initialize();

        if result.is_err()
        {
            uart.put_str("Failed to initialize block device.\n");
            uart.put_str("Error: ");
            uart.put_str(result.err().unwrap());
            uart.put_str("\n");

            panic!("");
        }
    }

    // Perform a polling read from the block device. We'll read a single 512 byte sector.
    pub fn read_sector(&mut self, sector: u64, buffer: &mut [u8; SECTOR_SIZE]) -> Result<(), &'static str>
    {
        self.virt_device.read_sector(sector, buffer)
    }

    // Finds a bootable partition on the block device. In this case we expect that the partition is
    // a fat32 partition. It's a very simple implementation that just returns the first fat32
    // partition it finds.
    //
    // If no fat32 partitions are found, it returns None.
    pub fn find_bootable_partition(&self, uart: &Uart) -> Option<Partition>
    {
        let mut buffer = [0u8; SECTOR_SIZE];

        let result = self.virt_device.read_sector(0, &mut buffer);

        if let Err(e) = result
        {
            uart.put_str("Failed to read sector 0 from block device.\n");

            uart.put_str("Error: ");
            uart.put_str(e);
            uart.put_str("\n");

            return None;
        }

        uart.put_str("Read sector 0 from block device.\n");

        // Check if the MBR is valid.

        let mbr = MasterBootRecord::new(&buffer);

            if mbr.is_valid() == false
            {
                uart.put_str("Invalid MBR found on block device.\n");
                return None;
            }
            else
            {
                uart.put_str("Valid MBR found on block device.\n");
            }

//            for partition in mbr.partition_entries.iter()
//            {
//                // Check if the partition is bootable and has a valid type.
//                if partition.status == 0x80 && partition.partition_type == 0x0B
//                {
//                    return Some(*partition);
//                }
//            }
        None
    }

    fn property_to_u32(prop_value: &[u8]) -> u32
    {
        if prop_value.len() == 4
        {
            let bytes = [prop_value[0], prop_value[1], prop_value[2], prop_value[3]];
            u32::from_be_bytes(bytes)
        }
        else
        {
            panic!("Invalid property length for u32 property value.");
        }
    }

    fn property_to_u64(prop_value: &[u8]) -> usize
    {
        if prop_value.len() == 8
        {
            let bytes =
                [
                    prop_value[0], prop_value[1], prop_value[2], prop_value[3],
                    prop_value[4], prop_value[5], prop_value[6], prop_value[7]
                ];

            usize::from_be_bytes(bytes)
        }
        else
        {
            panic!("Invalid property length for u64 property value.");
        }
    }

    fn is_compatible(prop_value: &[u8], target: &str) -> bool
    {
        prop_value
            .split(|&c| c == 0)
            .filter(|s| !s.is_empty())
            .any(|s|
                {
                    if let Ok(compatible_str) = str::from_utf8(s)
                    {
                        compatible_str == target
                    }
                    else
                    {
                        false
                    }
                })
    }
}
