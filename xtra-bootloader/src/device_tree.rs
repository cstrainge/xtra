
// Implementation of the Device Tree Blob (DTB) structure and methods to read and iterate through
// the device tree structure and properties.
//
// It's up the calling code to interpret the device tree blocks and their properties.

use core::{ mem::offset_of, ptr, slice::from_raw_parts, str::from_utf8_unchecked };

use crate::uart::Uart;



// Make sure that the device tree is valid by checking its magic number.
pub fn validate_dtb(device_tree_ptr: *const u8) -> bool
{
    let magic = unsafe { u32::from_be(ptr::read(device_tree_ptr as *const u32)) };

    if magic != 0xd00dfeed
    {
        return false;
    }

    // Additional checks can be added here, such as checking the version or size.

    true
}


// The DeviceTree structure represents the device tree blob (DTB) header and provides methods to
// read and iterate through the device tree structure and properties.
//
// Because we are running in a heapless we manage the device tree structure in place and do not
// allocate any memory for it. The device tree is expected to receive a pointer to the start of the
// device tree blob (DTB) in memory.
pub struct DeviceTree
{
    dtb_base: *const u8,      // Pointer to the start of the device tree blob.

    total_size: u32,          // Total size of DTB in bytes.
    off_dt_struct: u32,       // Offset to structure block.
    off_dt_strings: u32,      // Offset to strings block.
    off_mem_res_map: u32,     // Offset to memory reservation block.
    version: u32,             // DTB version (typically 17).
    last_comp_version: u32,   // Last compatible version (17).
    boot_cpu_id_phys: u32,    // Physical ID of boot CPU.
    size_dt_strings: u32,     // Length of strings block.
    size_dt_struct: u32,      // Length of structure block.
}


// Constants for the device tree structure markers.
const BEGIN_NODE: u32 = 0x0000_0001;  // Begin node marker.
const END_NODE: u32   = 0x0000_0002;  // End node marker.
const PROPERTY: u32   = 0x0000_0003;  // Property marker.
const NOP: u32        = 0x0000_0004;  // No operation marker.
const END: u32        = 0x0000_0009;  // End marker.


impl DeviceTree
{
    pub fn new(device_tree_ptr: *const u8) -> DeviceTree
    {
        // Get the pointer to the start of the device tree header, just past the magic number.
        // We're assuming that the magic number was already validated.
        let mut ptr: *const u32 = unsafe { (device_tree_ptr as *const u32).add(1) };

        // Read the device tree header fields.
        DeviceTree
        {
            dtb_base: device_tree_ptr,

            total_size: DeviceTree::read_u32(&mut ptr),
            off_dt_struct: DeviceTree::read_u32(&mut ptr),
            off_dt_strings: DeviceTree::read_u32(&mut ptr),
            off_mem_res_map: DeviceTree::read_u32(&mut ptr),
            version: DeviceTree::read_u32(&mut ptr),
            last_comp_version: DeviceTree::read_u32(&mut ptr),
            boot_cpu_id_phys: DeviceTree::read_u32(&mut ptr),
            size_dt_strings: DeviceTree::read_u32(&mut ptr),
            size_dt_struct: DeviceTree::read_u32(&mut ptr),
        }
    }

    // Read a 32-bit unsigned integer from the device tree header, assuming big-endian format.
    // The data_ptr is a mutable pointer to the current position in the device tree header.
    // It is updated to point to the next field after reading.
    fn read_u32(data_ptr: &mut *const u32) -> u32
    {
        unsafe
        {
            let value = u32::from_be(ptr::read_volatile(*data_ptr));

            *data_ptr = data_ptr.add(1);
            value
        }
    }

    // Print the device tree header information to the given UART.
    pub fn print_tree(&self, uart: &Uart)
    {
        uart.put_str("Device Tree Header:\n");

        Self::write_int(&uart, "  Version                            ", self.version);
        Self::write_int(&uart, "  Last Compatible Version            ", self.last_comp_version);
        Self::write_int(&uart, "  Total Size                         ", self.total_size);
        Self::write_hex(&uart, "  Offset to Structure Block          ", self.off_dt_struct);
        Self::write_hex(&uart, "  Offset to Strings Block            ", self.off_dt_strings);
        Self::write_hex(&uart, "  Offset to Memory Reservation Block ", self.off_mem_res_map);
        Self::write_int(&uart, "  Boot CPU ID (Physical)             ", self.boot_cpu_id_phys);
        Self::write_int(&uart, "  Size of Strings Block              ", self.size_dt_strings);
        Self::write_int(&uart, "  Size of Structure Block            ", self.size_dt_struct);

        // Let's print the contents of the DTB, this will help us understand what devices are
        // available in the system.
        self.iterate_blocks(|offset, name|
            {
                // Print the block information.
                uart.put_str("    Block: ");
                uart.put_str(name);
                uart.put_str("\n");

                self.iterate_properties(offset, |prop_name, prop_value|
                    {
                        uart.put_str("      Property: ");
                        uart.put_str(prop_name);
                        uart.put_str(", value: ");

                        if prop_value.is_empty()
                        {
                            uart.put_str("N/A");
                        }
                        else
                        {
                            uart.put_char(b'[');
                            uart.put_int(prop_value.len());
                            uart.put_str("] = ");
                            uart.put_hex_bytes(prop_value, Some(16));
                        }

                        uart.put_str("\n");

                        true
                    });

                true
            });
    }

    // Write an integer field value to the UART with a name.
    fn write_int(uart: &Uart, name: &str, value: u32)
    {
        uart.put_str(name);
        uart.put_str(": ");
        uart.put_int(value as usize);
        uart.put_str("\n");
    }


    // Write a hexadecimal field value to the UART with a name.
    fn write_hex(uart: &Uart, name: &str, value: u32)
    {
        uart.put_str(name);
        uart.put_str(": ");
        uart.put_hex(value as usize, true);
        uart.put_str("\n");
    }


    // Iterate through the device tree structure block, calling the callback for each found node.
    //
    // The callback receives the name of the node and the current byte offset in the structure
    // block. The callback can then use that offset to iterate through the properties of the node,
    // (if any.)
    pub fn iterate_blocks<Func>(&self, mut callback: Func)
        where
            Func: FnMut(usize, &str) -> bool
    {
        let mut current_offset = 0;

        let off_dt_struct = self.off_dt_struct as usize;
        let struct_ptr = unsafe { (self.dtb_base).add(off_dt_struct) as *const u8 };

        loop
        {
            // Read the next 32-bit word from the structure block.
            let word_ptr = unsafe { struct_ptr.add(current_offset) as *const u32 };
            let word = unsafe { u32::from_be(ptr::read_volatile(word_ptr)) };

            match word
            {
                BEGIN_NODE =>
                    {
                        // We're at the beginning of a node, so we need to read the node name.
                        // The format of a node marker is:
                        // 1. Node marker (4 bytes)
                        // 2. Node name string, padded to a 4-byte boundary.
                        // 3. Property markers or end node marker.

                        // Move past the node marker.
                        self.increment_offset(&mut current_offset, 4);

                        // Get a pointer to the node name offset string.
                        let name_ptr = unsafe { (struct_ptr).add(current_offset) };

                        // Convert the name pointer to a string.
                        let ( node_name, name_size ) = self.extract_node_name_to_buffer(name_ptr);

                        // Move past the node name string plus the padding.
                        self.increment_offset(&mut current_offset, name_size);

                        // Call the callback with the node name and current offset.
                        if !callback(current_offset, node_name)
                        {
                            // If the callback returns false, we stop iterating.
                            break;
                        }
                    },

                END_NODE =>
                    {
                        // We've reached the end of a node, so we can skip to the next word.
                        self.increment_offset(&mut current_offset, 4);
                    },

                PROPERTY =>
                    {
                        // We're at a property marker, we need to skip past it to the next marker.
                        // The format of a property is:
                        // 1. Property marker (4 bytes)
                        // 2. Property size (4 bytes)
                        // 3. Property name offset (4 bytes)
                        // 4. Property value (variable length)

                        // Move past the property marker.
                        self.increment_offset(&mut current_offset, 4);

                        // Get a pointer to the property size.
                        let prop_size_ptr = unsafe { struct_ptr.add(current_offset) as *const u32 };

                        // Read the property size from big-endian format.
                        let prop_size = unsafe { ptr::read_volatile(prop_size_ptr) };
                        let prop_size = u32::from_be(prop_size);

                        // Move past the property size and the name offset.
                        self.increment_offset(&mut current_offset, 8);

                        // Move past the property value data, which is padded to a 4-byte boundary.
                        self.increment_offset(&mut current_offset, prop_size as usize);
                    },

                NOP =>
                    {
                        // No operation marker, just skip it.
                        self.increment_offset(&mut current_offset, 4);
                    },

                END =>
                    {
                        // End of structure block, break out of the loop.
                        break;
                    },

                _ =>
                    {
                        // Unknown marker, just skip it.
                        self.increment_offset(&mut current_offset, 4);
                    }

            }
        }
    }


    pub fn iterate_properties<Func>(&self, base_offset: usize, mut callback: Func)
        where
            Func: FnMut(&str, &[u8]) -> bool
    {
        let mut current_offset = base_offset;

        // Get a pointer to the start of the structure block's properties.
        let off_dt_struct = self.off_dt_struct as usize;
        let struct_ptr = unsafe { (self.dtb_base).add(off_dt_struct) as *const u8 };

        loop
        {
            // Read the next 32-bit word from the structure block.
            let word_ptr = unsafe { struct_ptr.add(current_offset) as *const u32 };
            let word = unsafe { u32::from_be(ptr::read_volatile(word_ptr)) };

            match word
            {
                BEGIN_NODE =>
                {
                    // We're at a beginning of a new node, so break out of the loop.
                    break;
                },

                END_NODE =>
                {
                    // We've reached the end of a node, so we can stop iterating properties.
                    break;
                },

                PROPERTY =>
                {
                    // Break down the property structure and call the callback with the property's
                    // name and value.

                    // The format of a property is:
                    // 1. Property marker (4 bytes)
                    // 2. Property size (4 bytes)
                    // 3. Property name offset (4 bytes)
                    // 4. Property value (variable length)

                    // Move past the property marker.
                    self.increment_offset(&mut current_offset, 4);

                    // Get the property data size.
                    let prop_size_ptr = unsafe { struct_ptr.add(current_offset) as *const u32 };

                    let prop_size = unsafe { ptr::read_volatile(prop_size_ptr) };
                    let prop_size = u32::from_be(prop_size);

                    self.increment_offset(&mut current_offset, 4);

                    // Get the property name string offset.
                    let prop_name_offset_ptr = unsafe
                        {
                            struct_ptr.add(current_offset) as *const u32
                        };

                    let prop_name_offset = unsafe { ptr::read_volatile(prop_name_offset_ptr) };
                    let prop_name_offset = u32::from_be(prop_name_offset);

                    self.increment_offset(&mut current_offset, 4);

                    // Get the bytes of the property value.
                    let prop_value_ptr = unsafe { struct_ptr.add(current_offset) as *const u8 };

                    // Get the property value as a slice.
                    let prop_value = unsafe
                        {
                            from_raw_parts(prop_value_ptr, prop_size as usize)
                        };

                    // Move past the property value data, which is padded to a 4-byte boundary.
                    self.increment_offset(&mut current_offset, prop_size as usize);

                    // Get the property name string from the strings block.
                    let off_dt_strings = self.off_dt_strings as usize;

                    let name_ptr = unsafe
                        {
                            (self.dtb_base).add(off_dt_strings + prop_name_offset as usize)
                        };

                    let ( prop_name, _ ) = self.extract_node_name_to_buffer(name_ptr);

                    // Call the callback with the property name and value.
                    if !callback(prop_name, prop_value)
                    {
                        // If the callback returns false, we stop iterating.
                        break;
                    }
                },

                NOP =>
                {
                    // No operation marker, just skip it.
                    self.increment_offset(&mut current_offset, 4);
                },

                END =>
                {
                    // End of structure block, break out of the loop.
                    break;
                },

                _ =>
                {
                    // Unknown marker, just skip it.
                    self.increment_offset(&mut current_offset, 4);
                }
            }
        }
    }


    // Move through the device tree structure block, making sure that we don't read past the end
    // of the data structure. Panic if we do.
    fn increment_offset(&self, offset: &mut usize, size: usize)
    {
        // Increment the offset by the given size, ensuring it is aligned to a 4-byte boundary.
        *offset += (size + 3) & !3;

        if *offset as u32 >= self.size_dt_struct
        {
            panic!("Attempted to read past the end of the device tree structure block.");
        }
    }


    // Create a string reference from the bytes in the device tree structure block at the given
    // pointer. The string is expected to be null-terminated.
    //
    // Return the string reference and the size of the string including the null terminator.
    fn extract_node_name_to_buffer(&self, name_ptr: *const u8) -> ( &str, usize )
    {
        const SIZE: usize = 256;
        static mut NAME_BUFFER: [u8; SIZE] = [0; SIZE];

        unsafe
        {
            let mut i = 0;

            while    i < SIZE - 1
                  && unsafe { *name_ptr.add(i) } != 0
            {
                // Copy the byte from the name pointer to the buffer.
                NAME_BUFFER[i] = unsafe { *name_ptr.add(i) };

                // Move to the next byte in the name pointer and increment the index.
                i += 1;
            }

            let node_name = from_utf8_unchecked(&NAME_BUFFER[0..i]);

            ( node_name, i + 1 )
        }
    }
}
