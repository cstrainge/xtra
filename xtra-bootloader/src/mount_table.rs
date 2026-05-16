
// Module to parse the mount table from the boot drive for passing to the Kernel.

use xtra_kernel_shared::mount_table::{ XtraFilesystemType,
                                       XtraMountTable,
                                       XTRA_MAX_MOUNT_TABLE_ENTRIES,
                                       XTRA_MAX_MOUNT_POINT_STRING_LENGTH };

use crate::fat32::{ DirectoryEntry, DirectoryIterator, Fat32Volume, FileStream };



/// The name of the mount table as expected in the boot drive's root directory.
const MOUNT_TABLE_FILE_NAME: &[u8; 11] = b"MOUNT   TBL";


/// Consume the file stream until we have a non-whitespace character or the end of the stream.
fn skip_whitespace(file_stream: &mut FileStream) -> Result<(), &'static str>
{
    while !file_stream.is_eof()
    {
        let position = file_stream.tell();
        let next = file_stream.read_u8()?;

        if !next.is_ascii_whitespace()
        {
            file_stream.seek(position)?;
            break;
        }
    }

    Ok(())
}


/// Expect the provided string to be present at the current position in the mount stream. If we find
/// text that isn't the string that's considered an error. But if we reach the end of the stream
/// then we return not found. If the string matches then we return found.
///
/// If the function fails to match the expected string it will rewind the stream back to the
/// position it started at.
fn expect_string(file_stream: &mut FileStream, expected_string: &str) -> Result<bool, &'static str>
{
    let mut current_byte: usize = 0;
    let position = file_stream.tell();

    while    !file_stream.is_eof()
          && (current_byte < expected_string.len())
    {
        let next = file_stream.read_u8()?;

        if next != expected_string.as_bytes()[current_byte]
        {
            file_stream.seek(position)?;
            return Ok(false);
        }

        current_byte += 1;
    }

    if current_byte < expected_string.len()
    {
        file_stream.seek(position)?;
        Ok(false)
    }
    else
    {
        Ok(true)
    }
}


/// Read characters from the stream until we hit whitespace, the end of the stream,
/// or the end of the buffer.
fn read_until_whitespace(file_stream: &mut FileStream,
                         buffer: &mut [u8]) -> Result<(), &'static str>
{
    let mut buffer_index: usize = 0;

    while    !file_stream.is_eof()
          && (buffer_index < buffer.len())
    {
        let position = file_stream.tell();
        let next = file_stream.read_u8()?;

        if !next.is_ascii_whitespace()
        {
            buffer[buffer_index] = next;
            buffer_index += 1;
        }
        else
        {
            file_stream.seek(position)?;
            break;
        }
    }

    Ok(())
}


/// Read a numeric uint8 from the text stream. The number is expected to be base 10.
fn read_number(file_stream: &mut FileStream) -> Result<u8, &'static str>
{
    let mut buffer: [u8; 3] = [0; 3];
    let mut number: u16 = 0;

    read_until_whitespace(file_stream, &mut buffer)?;

    for byte in buffer.iter()
    {
        if *byte == 0
        {
            break;
        }

        if !byte.is_ascii_digit()
        {
            return Err("Expected a number but found non-digit characters.");
        }

        let digit_value = (byte - b'0') as u16;
        number = number * 10 + digit_value;

        if number > (u8::MAX as u16)
        {
            return Err("Number is too large to fit in a u8.");
        }
    }

    Ok(number as u8)
}


/// Read the filesystem type from the mount stream. We expect to see either "fat32" or "ext2" here.
/// If an unknown filesystem type is found we consume the token and return it as unknown.
fn read_mount_type_enum(file_stream: &mut FileStream) -> Result<XtraFilesystemType, &'static str>
{
    let mount_type = XtraFilesystemType::Unknown;

    skip_whitespace(file_stream)?;

    // Check to see if we're fat32?
    let fat32_found = expect_string(file_stream, "fat32")?;

    if fat32_found
    {
        return Ok(XtraFilesystemType::Fat32);
    }

    // Ok, look for ext2 next.
    let ext2_found = expect_string(file_stream, "ext2")?;

    if ext2_found
    {
        return Ok(XtraFilesystemType::Ext2);
    }

    // Ok, we have no idea of what the filesystem type is, so we just try to consume the next token
    // and record it as unknown.
    let mut buffer: [u8; 16] = [0; 16];
    read_until_whitespace(file_stream, &mut buffer)?;

    Ok(mount_type)
}


/// Given a Fat-32 file stream for the mount table, attempt to parse it and populate the provided
/// mount table structure.
fn parse_file_stream(file_stream: &mut FileStream,
                      mount_table: &mut XtraMountTable) -> Result<(), &'static str>
{
    let mut entry_index: usize = 0;

    loop
    {
        // Make sure we're still within the bounds of the mount table entries.
        if entry_index >= XTRA_MAX_MOUNT_TABLE_ENTRIES
        {
            break;
        }

        // Allow whitespace before the "mount" keyword for each entry.
        skip_whitespace(file_stream)?;

        // All mount table entries start with the "mount" keyword so we expect to see that at the
        // beginning of each entry. If we don't see it then we've reached the end of the mount table
        // entries.
        let found = expect_string(file_stream, "mount")?;

        if !found
        {
            break;
        }

        // Skip any whitespace after the "mount" keyword.
        skip_whitespace(file_stream)?;

        // The next token should be the mount point and it always starts with a "/" character.
        let mount_point_found = expect_string(file_stream, "/")?;

        if !mount_point_found
        {
            return Err("Expected mount point starting with '/' in mount table entry.");
        }

        // Read the mount string from the stream until we hit whitespace.
        let mut mount_string: [u8; XTRA_MAX_MOUNT_POINT_STRING_LENGTH]
            = [0; XTRA_MAX_MOUNT_POINT_STRING_LENGTH];

        mount_string[0] = b'/';
        read_until_whitespace(file_stream, &mut mount_string[1..])?;

        mount_table.entries[entry_index].mount_point = mount_string;

        // Skip any whitespace after the mount string.
        skip_whitespace(file_stream)?;

        // Now look for the "disk:" keyword which indicates the device ID for this mount entry.
        let disk_id_found = expect_string(file_stream, "disk:")?;

        if !disk_id_found
        {
            return Err("Expected 'disk:' keyword in mount table entry.");
        }

        let disk_index = read_number(file_stream)?;

        mount_table.entries[entry_index].device = disk_index;
        skip_whitespace(file_stream)?;

        // Now we're looking for the partition ID which starts with the "pt:" keyword.
        let partition_id_found = expect_string(file_stream, "pt:")?;

        if !partition_id_found
        {
            return Err("Expected 'pt:' keyword in mount table entry.");
        }

        let partition_index = read_number(file_stream)?;
        mount_table.entries[entry_index].partition = partition_index;
        skip_whitespace(file_stream)?;

        // Finally we look for the filesystem type which starts with the "type:" keyword.
        let filesystem_type_found = expect_string(file_stream, "type:")?;

        if !filesystem_type_found
        {
            return Err("Expected 'type:' keyword in mount table entry.");
        }

        let type_id = read_mount_type_enum(file_stream)?;
        mount_table.entries[entry_index].filesystem_type = type_id;

        // We've definitely found a new entry.
        mount_table.num_entries += 1;

        // Advance to the next entry index.
        entry_index += 1;
    }

    Ok(())
}


/// Attempt to find and load the mount table from the specified directory iterator. If we can't find
/// the mount table we just return an empty mount table to the caller.
pub fn load_mount_table(volume: &Fat32Volume,
                        directory_iterator: &mut DirectoryIterator)
                        -> Result<XtraMountTable, &'static str>
{
    let mut mount_table = XtraMountTable::default();
    let mut table_entry: Option<DirectoryEntry> = None;

    let result = directory_iterator.iterate(|entry|
        {
            // Loop through the directory entries until we find the mount table or we exhaust the
            // directory entries.
            if    entry.is_file()
               && entry.name == *MOUNT_TABLE_FILE_NAME
            {
                table_entry = Some(entry.clone());
                false
            }
            else
            {
                true
            }
        });

    // Make sure that the directory iteration succeeded. We still have to account for filesystem
    // errors here even if the iteration itself succeeded.
    if let Err(error) = result
    {
        return Err(error);
    }

    // Check to see if we found the mount table in the root directory. If we didn't we just return
    // a blank mount table to the kernel and let it deal with the fact that we don't have any mount
    // points defined.
    if let Some(table_entry) = table_entry
    {
        // We have a directory entry for the mount table so attempt to create a file stream for it.
        let mount_stream = FileStream::new_from_directory_entry(volume, &table_entry);

        if let Err(error) = mount_stream
        {
            return Err(error);
        }

        // Parse the mount table from the file stream and return it to the caller.
        let mut mount_stream = mount_stream.unwrap();

        parse_file_stream(&mut mount_stream, &mut mount_table)?;
    }

    Ok(mount_table)
}
