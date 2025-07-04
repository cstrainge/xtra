
use core::{ mem::size_of, ptr::{ read_volatile, write_volatile }, str };

use crate::device_tree::DeviceTree;



// Represents the range of registers for a block device.
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



// VirtIO MMIO Register Offsets.
const VIRTIO_MMIO_MAGIC_VALUE:         usize = 0x000;  // Magic "virt" (0x74726976.)
const VIRTIO_MMIO_VERSION:             usize = 0x004;  // Version (should be 2.)
const VIRTIO_MMIO_DEVICE_ID:           usize = 0x008;  // Device Type (2 = block device.)
const VIRTIO_MMIO_VENDOR_ID:           usize = 0x00c;  // Vendor ID.
const VIRTIO_MMIO_DEVICE_FEATURES:     usize = 0x010;  // Device features (read).
const VIRTIO_MMIO_DEVICE_FEATURES_SEL: usize = 0x014;  // Feature selection.
const VIRTIO_MMIO_DRIVER_FEATURES:     usize = 0x020;  // Driver features (write.)
const VIRTIO_MMIO_DRIVER_FEATURES_SEL: usize = 0x024;  // Driver feature selection.
const VIRTIO_MMIO_QUEUE_SEL:           usize = 0x030;  // Queue select.
const VIRTIO_MMIO_QUEUE_NUM_MAX:       usize = 0x034;  // Maximum queue size.
const VIRTIO_MMIO_QUEUE_NUM:           usize = 0x038;  // Queue size.
const VIRTIO_MMIO_QUEUE_READY:         usize = 0x044;  // Queue ready bit.
const VIRTIO_MMIO_QUEUE_NOTIFY:        usize = 0x050;  // Queue notify.
const VIRTIO_MMIO_INTERRUPT_STATUS:    usize = 0x060;  // Interrupt status.
const VIRTIO_MMIO_INTERRUPT_ACK:       usize = 0x064;  // Interrupt acknowledge.
const VIRTIO_MMIO_STATUS:              usize = 0x070;  // Device status.
const VIRTIO_MMIO_QUEUE_DESC_LOW:      usize = 0x080;  // Queue descriptor table address (low.)
const VIRTIO_MMIO_QUEUE_DESC_HIGH:     usize = 0x084;  // Queue descriptor table address (high.)
const VIRTIO_MMIO_QUEUE_AVAIL_LOW:     usize = 0x090;  // Queue available ring address (low.)
const VIRTIO_MMIO_QUEUE_AVAIL_HIGH:    usize = 0x094;  // Queue available ring address (high.)
const VIRTIO_MMIO_QUEUE_USED_LOW:      usize = 0x0a0;  // Queue used ring address (low.)
const VIRTIO_MMIO_QUEUE_USED_HIGH:     usize = 0x0a4;  // Queue used ring address (high.)



// VirtIO Status Bits.
const VIRTIO_STATUS_ACKNOWLEDGE: u32 = 0b_0000_0001;
const VIRTIO_STATUS_DRIVER:      u32 = 0b_0000_0010;
const VIRTIO_STATUS_DRIVER_OK:   u32 = 0b_0000_0100;
const VIRTIO_STATUS_FEATURES_OK: u32 = 0b_0000_1000;
const VIRTIO_STATUS_FAILED:      u32 = 0b_1000_0000;



// VirtIO Queue Descriptor Flags.
const VIRT_Q_DESC_F_NEXT:  u16 = 1;     // Descriptor continues via next field.
const VIRT_Q_DESC_F_WRITE: u16 = 2;     // Device writes (vs reads.)



// VirtIO Queue Constants.
const QUEUE_SIZE:  usize = 16;          // Small queue for bootloader.
const QUEUE_ALIGN: usize = 4096;        // Page alignment requirement.



// VirtIO Block Request Types.
const VIRTIO_BLK_T_IN: u32 = 0;        // Read request.
const VIRTIO_BLK_T_OUT: u32 = 1;       // Write request.



const SECTOR_SIZE: usize = 512;        // Standard sector size for block devices.



#[repr(C)]
#[derive(Clone, Copy)]
struct VirtIoDescriptor
{
    address: u64,  // Address of the data buffer.
    length: u32,   // Length of the data buffer.
    flags: u16,    // Flags for the descriptor.
    next: u16      // Next descriptor index (for chained descriptors).
}


impl VirtIoDescriptor
{
    pub fn new() -> Self
    {
        VirtIoDescriptor
            {
                address: 0,
                length: 0,
                flags: 0,
                next: 0
            }
    }

    pub fn get_address(&self) -> u64
    {
        unsafe { read_volatile(&self.address) }
    }

    pub fn set_address(&mut self, address: u64)
    {
        unsafe { write_volatile(&mut self.address, address) };
    }

    pub fn get_length(&self) -> u32
    {
        unsafe { read_volatile(&self.length) }
    }

    pub fn set_length(&mut self, length: u32)
    {
        unsafe { write_volatile(&mut self.length, length) };
    }

    pub fn get_flags(&self) -> u16
    {
        unsafe { read_volatile(&self.flags) }
    }

    pub fn set_flags(&mut self, flags: u16)
    {
        unsafe { write_volatile(&mut self.flags, flags) };
    }
}



#[repr(C)]
struct VirtIoAvailable
{
    flags: u16,               // Flags for the available ring.
    index: u16,               // Index of the next available descriptor.
    ring: [u16; QUEUE_SIZE],  // Ring of available descriptor indices.
    used_event: u16           // Event index for used descriptors.
}



#[repr(C)]
#[derive(Clone, Copy)]
struct VirtIoUsedElement
{
    id: u32,          // Descriptor ID.
    length: u32       // Length of the data processed.
}



#[repr(C)]
struct VirtIoUsed
{
    flags: u16,                             // Flags for the used ring.
    index: u16,                             // Index of the next used descriptor.
    ring: [VirtIoUsedElement; QUEUE_SIZE],  // Ring of used descriptors.
    available_event: u16                    // Event index for available descriptors.
}



#[repr(C)]
struct VirtIoBlockRequest
{
    operation: u32,  // Operation type (read/write).
    reserved: u32,   // Reserved for future use.
    sector: u64,     // Sector number to read/write.
    status: u8       // Status of the operation (0 = success, non-zero = error).
}



// Representation of a storage device in the system.
//
// This could be a hard drive, SSD, or any other block device that can be used to store data. The
// bootloader will use this to find the kernel image and load it.
pub struct BlockDevice
{
    registers: Registers,                              // The register set for the block device.
    interrupts: u32,                                   // Interrupts for the device.
    interrupt_parent: u32,                             // Parent interrupt controller.

    queue_descriptor: [VirtIoDescriptor; QUEUE_SIZE],  // Queue descriptors.
    queue_available: VirtIoAvailable,                  // Available ring.
    queue_used: VirtIoUsed,                            // Used ring.

    available_index: u16,                              // Index of the next available descriptor.
    last_used_index: u16                               // Index of the next used descriptor.
}



// Represents a partition on a block device.
pub struct Partition
{
    // The partition's start address in the block device.
    start: usize,

    // The partition's size in bytes.
    size: usize
}



// The magic value used to identify VirtIO devices.
const VIRT_DEVICE_MAGIC: u32 = u32::from_le_bytes(*b"virt");



impl BlockDevice
{


    fn new(registers: Registers, interrupts: u32, interrupt_parent: u32) -> Self
    {
        BlockDevice
            {
                registers,
                interrupts,
                interrupt_parent,

                queue_descriptor:
                    [
                        VirtIoDescriptor
                        {
                            address: 0,
                            length: 0,
                            flags: 0,
                            next: 0
                        };
                        QUEUE_SIZE
                    ],

                queue_available: VirtIoAvailable
                    {
                        flags: 0,
                        index: 0,
                        ring: [0; QUEUE_SIZE],
                        used_event: 0
                    },

                queue_used: VirtIoUsed
                    {
                        flags: 0,
                        index: 0,
                        ring: [VirtIoUsedElement { id: 0, length: 0 }; QUEUE_SIZE],
                        available_event: 0
                    },

                available_index: 0,
                last_used_index: 0
            }
    }


    pub fn find_first_drive(device_tree: DeviceTree) -> Option<BlockDevice>
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
                        let device = BlockDevice::new(registers, interrupts, interrupt_parent);

                        if Self::probe_device(&device)
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
    pub fn initialize(&self)
    {
        // Reset the device by writing 0 to the status register.
        self.write_status(0);

        self.add_status(VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

        // Check the device's version.
        let _device_features = self.read_device_features();

        // Set features ok status.
        self.add_status(VIRTIO_STATUS_FEATURES_OK);

        if self.read_status() & VIRTIO_STATUS_FEATURES_OK == 0
        {
            panic!("Failed to set features ok status.");
        }

        // Setup the device queues.
        self.setup_queues();

        // All done, so we can set the driver ok status.
        self.add_status(VIRTIO_STATUS_DRIVER_OK);
    }

    // Perform a polling read from the block device. We'll read a single 512 byte sector.
    pub fn read_sector(&mut self, sector: u64, buffer: &mut [u8; SECTOR_SIZE]) -> Result<(), ()>
    {
        // Increment the available index to point to the next available descriptor within the queue.
        // Wrapping by queue size.
        fn next_index(index: usize) -> u16
        {
            (index % QUEUE_SIZE) as u16
        }

        // VirtIO block requests need 3 descriptors:
        // 1. The request header, sector number and operation type.
        // 2. The data buffer to write to.
        // 3. A status byte for the operation result.

        let mut request = VirtIoBlockRequest
            {
                operation: VIRTIO_BLK_T_IN,  // Read operation.
                sector: sector,
                reserved: 0,
                status: 0
            };

        // Setup the descriptor chain of 3 descriptors.
        let mut status: u8 = 0;
        let descriptor_index = self.available_index as usize;

        // Descriptor 1: The request header.
        self.queue_descriptor[descriptor_index] = VirtIoDescriptor
            {
                address: &request as *const _ as u64,
                length: size_of::<VirtIoBlockRequest>() as u32,
                flags: VIRT_Q_DESC_F_NEXT,
                next: next_index(descriptor_index + 1)
            };

        // Descriptor 2: The data buffer for the device to write our data to.
        self.queue_descriptor[next_index(descriptor_index + 1) as usize] = VirtIoDescriptor
            {
                address: buffer.as_mut_ptr() as u64,
                length: SECTOR_SIZE as u32,
                flags: VIRT_Q_DESC_F_WRITE | VIRT_Q_DESC_F_NEXT,  // Device writes + chain.
                next: next_index(descriptor_index + 2)
            };

        // Descriptor 3: The status byte for the operation result.
        self.queue_descriptor[next_index(descriptor_index + 2) as usize] = VirtIoDescriptor
            {
                address: &mut status as *mut _ as u64,
                length: size_of::<u8>() as u32,
                flags: VIRT_Q_DESC_F_WRITE,  // This is a write operation.
                next: 0  // No next descriptor.
            };

        // Write the descriptors to the queue.
        self.queue_available.ring[self.available_index as usize % QUEUE_SIZE]
            = descriptor_index as u16;

        self.queue_available.index = next_index((self.queue_available.index + 1) as usize);

        // Notify the device that there are available descriptors.
        self.notify_queue(0);

        // Wait for the device to process the request we poll in this implementation, the kernel
        // will use an interrupt to be notified when the request completes.
        while self.last_used_index == self.read_queue_used_index()
        {
            // Spin until the device has processed the request.
        }

        // Update the available index to point to the next available descriptor.
        self.last_used_index = self.read_queue_used_index();
        self.available_index = next_index((self.available_index + 3) as usize);

        // Check the status of the operation.
        if status == 0 { Ok(()) } else { Err(()) }
    }

    // Sets up the device queues for communication with the block device.
    fn setup_queues(&self)
    {
        // Select the first queue (index 0).
        self.write_queue_select(0);

        // Check the maximum queue size.
        let max_queue_size = self.read_queue_size_max();

        if max_queue_size == 0
        {
            panic!("Invalid maximum queue size.");
        }

        if max_queue_size < QUEUE_SIZE as u32
        {
            panic!("Queue size is too small,");
        }

        // Set the active size.
        self.write_queue_size(QUEUE_SIZE as u32);

        // Set Queue addresses.
        self.write_queue_descriptor_address(self.queue_descriptor.as_ptr());
        self.write_queue_available_address(&self.queue_available);
        self.write_queue_used_address(&self.queue_used);

        // Enable the queue.
        self.write_queue_ready(1);
    }

    // Finds a bootable partition on the block device. In this case we expect that the partition is
    // a fat32 partition. It's a very simple implementation that just returns the first fat32
    // partition it finds.
    //
    // If no fat32 partitions are found, it returns None.
    pub fn find_bootable_partition(&self) -> Option<Partition>
    {
        None
    }

    fn probe_device(device: &BlockDevice) -> bool
    {
        // Check if the device has a valid registers base and size.
        if device.registers.base == 0 || device.registers.size == 0
        {
            // Invalid device register set, so we can't use this device.
            return false;
        }

        if device.registers.base < 0x1000_0000
        {
            return false;
        }

        // Check if the magic value matches the VirtIO magic value.
        if device.read_magic() != VIRT_DEVICE_MAGIC
        {
            // Not a VirtIO device, so we can't use it.
            return false;
        }

        // Check if the device type is a block device.
        device.read_device_id() == 2
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

    fn read_magic(&self) -> u32
    {
        let magic_ptr = (self.registers.base + VIRTIO_MMIO_MAGIC_VALUE) as *const u32;
        unsafe { read_volatile(magic_ptr) }
    }

    fn read_version(&self) -> u32
    {
        let version_ptr = (self.registers.base + VIRTIO_MMIO_VERSION) as *const u32;
        unsafe { read_volatile(version_ptr) }
    }

    fn read_device_id(&self) -> u32
    {
        let device_id_ptr = (self.registers.base + VIRTIO_MMIO_DEVICE_ID) as *const u32;
        unsafe { read_volatile(device_id_ptr) }
    }

    fn read_status(&self) -> u32
    {
        let status_ptr = (self.registers.base + VIRTIO_MMIO_STATUS) as *const u32;
        unsafe { read_volatile(status_ptr) }
    }

    fn write_status(&self, status: u32)
    {
        let status_ptr = (self.registers.base + VIRTIO_MMIO_STATUS) as *mut u32;
        unsafe { write_volatile(status_ptr, status); }
    }

    fn add_status(&self, status: u32)
    {
        let current_status = self.read_status();
        self.write_status(current_status | status);
    }

    fn read_device_features(&self) -> u32
    {
        // Select feature bits 0-31.
        let sel_ptr = (self.registers.base + VIRTIO_MMIO_DEVICE_FEATURES_SEL) as *mut u32;
        unsafe { core::ptr::write_volatile(sel_ptr, 0) };

        // Read the device features.
        let features_ptr = (self.registers.base + VIRTIO_MMIO_DEVICE_FEATURES) as *const u32;
        unsafe { read_volatile(features_ptr) }
    }

    fn read_queue_used_index(&self) -> u16
    {
        // Read teh used index from the used ring data structure. We perform a volatile read because
        // this value can change at any time by the device.
        let used_index_ptr = &self.queue_used.index as *const u16;
        unsafe { read_volatile(used_index_ptr) }
    }

    fn write_queue_select(&self, queue_index: u32)
    {
        let queue_sel_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_SEL) as *mut u32;
        unsafe { write_volatile(queue_sel_ptr, queue_index); }
    }

    fn read_queue_size_max(&self) -> u32
    {
        let queue_num_max_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_NUM_MAX) as *const u32;
        unsafe { read_volatile(queue_num_max_ptr) }
    }

    fn write_queue_size(&self, size: u32)
    {
        let queue_num_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_NUM) as *mut u32;
        unsafe { write_volatile(queue_num_ptr, size); }
    }

    fn notify_queue(&self, queue_index: u32)
    {
        let notify_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_NOTIFY) as *mut u32;
        unsafe { write_volatile(notify_ptr, queue_index); }
    }

    fn write_queue_ready(&self, ready: u32)
    {
        let queue_ready_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_READY) as *mut u32;
        unsafe { write_volatile(queue_ready_ptr, ready); }
    }

    fn read_queue_num(&self) -> u32
    {
        let queue_num_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_NUM) as *const u32;
        unsafe { read_volatile(queue_num_ptr) }
    }

    fn write_queue_descriptor_address(&self, address: *const VirtIoDescriptor)
    {
        // The descriptor address is a 64-bit value, so we need to write it in two parts.
        let address = address as usize;

        // Get the low and high parts of the address.
        let desc_low_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_DESC_LOW) as *mut u32;
        let desc_high_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_DESC_HIGH) as *mut u32;

        // Write the low and high parts of the address to the MMIO registers.
        unsafe
        {
            write_volatile(desc_low_ptr, address as u32);
            write_volatile(desc_high_ptr, (address >> 32) as u32);
        }
    }

    fn write_queue_available_address(&self, address: *const VirtIoAvailable)
    {
        // The available address is a 64-bit value, so we need to write it in two parts.
        let address = address as usize;

        // Get the low and high parts of the address.
        let avail_low_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_AVAIL_LOW) as *mut u32;
        let avail_high_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_AVAIL_HIGH) as *mut u32;

        // Write the low and high parts of the address to the MMIO registers.
        unsafe
        {
            write_volatile(avail_low_ptr, address as u32);
            write_volatile(avail_high_ptr, (address >> 32) as u32);
        }
    }

    fn write_queue_used_address(&self, address: *const VirtIoUsed)
    {
        // The used address is a 64-bit value, so we need to write it in two parts.
        let address = address as usize;

        // Get the low and high parts of the address.
        let used_low_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_USED_LOW) as *mut u32;
        let used_high_ptr = (self.registers.base + VIRTIO_MMIO_QUEUE_USED_HIGH) as *mut u32;

        // Write the low and high parts of the address to the MMIO registers.
        unsafe
        {
            write_volatile(used_low_ptr, address as u32);
            write_volatile(used_high_ptr, (address >> 32) as u32);
        }
    }
}
