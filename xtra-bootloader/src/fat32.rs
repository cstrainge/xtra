
// All of our code for managing the FAT32 filesystem. We can iterate the root directory of a given
// FAT32 filesystem on a given partition of a block device. This code is used to find and stream in
// the kernel file from the filesystem.

use core::slice::from_raw_parts_mut;

use crate::{ block_device::{ BlockDevice, SECTOR_SIZE },
             partition_table::{ MasterBootRecord, LegacyPartition } };



// We can return an iterator that iterates over the bytes of the kernel file or an error if the
// kernel file is not found or the FAT32 volume is invalid.
type FatResult<T> = Result<T, &'static str>;



type SectorBuffer = [u8; SECTOR_SIZE];  // A buffer for reading sectors from the block device.



const SECTOR_CACHE_SIZE: usize = 4;  // Number of sectors to cache in memory for faster access.



// Keep a cache of buffers for loading sectors from the block device.
struct SectorCache
{
    sectors: [SectorBuffer; SECTOR_CACHE_SIZE],  // Cached sectors.
    used: [bool; SECTOR_CACHE_SIZE],             // Dirty flags for each cached sector.
    index: usize                                 // Index of the next sector to use.
}



impl SectorCache
{
    pub const fn new() -> Self
    {
        SectorCache
            {
                sectors: [[0; SECTOR_SIZE]; SECTOR_CACHE_SIZE],
                used: [false; SECTOR_CACHE_SIZE],
                index: 0
            }
    }

    // Get a sector from the cache or read it from the block device if not cached.
    pub fn get_buffer(&mut self) -> (usize, &mut SectorBuffer)
    {
        for i in 0..SECTOR_CACHE_SIZE
        {
            let index = (self.index + i) % SECTOR_CACHE_SIZE;

            if !self.used[index]
            {
                self.used[index] = true;
                self.index = index;

                return (index, &mut self.sectors[index]);
            }
        }

        panic!("");
    }

    pub fn free_buffer(&mut self, index: usize)
    {
        if index >= SECTOR_CACHE_SIZE
        {
            panic!("");
        }

        assert!(self.used[index]);

        self.used[index] = false;
    }
}



// Our global single threaded sector cache for dealing with the filesystem.
static mut SECTOR_CACHE: SectorCache = SectorCache::new();



// Get a sector buffer from the cache. This function returns a tuple containing the index of the
// buffer in the cache and a mutable reference to the buffer itself. The caller is responsible for
// freeing the buffer when done with it.
fn get_sector_buffer() -> (usize, &'static mut SectorBuffer)
{
    unsafe { SECTOR_CACHE.get_buffer() }
}



// Free a sector buffer by its index. This function is used to release a buffer back to the cache
// after it has been used. The index must be valid and within the range of the cache.
fn free_sector_buffer(index: usize)
{
    unsafe { SECTOR_CACHE.free_buffer(index) }
}



// Implementation of a simple defer mechanism that allows us to run a closure when a sector buffer
// goes out of scope and ensures that it is freed properly.
struct Defer<F: FnOnce()>
{
    f: Option<F>
}

impl<F: FnOnce()> Defer<F>
{
    fn new(f: F) -> Self
    {
        Defer { f: Some(f) }
    }
}

impl<F: FnOnce()> Drop for Defer<F>
{
    fn drop(&mut self)
    {
        if let Some(f) = self.f.take()
        {
            f();
        }
    }
}



// The file allocation table (FAT) structure. The FAT is where all the chains of file clusters are
// managed. For every cluster in the filesystem, it's cluster entry indicates the next cluster in
// the chain for that file or directory. If there are no more clusters in the chain, the entry is
// set to a special end-of-chain marker.
//
// Note that this implementation uses a static buffer for the FAT entries. This puts an upper limit
// on the size of the filesystem we can handle in this bootloader.
//
// The offshoot of this implementation is that we can only mount one FAT32 filesystem at a time.
//
// The maximum size of the file system we can handle is calculated by the number of entries in the
// FAT table multiplied by the number of sectors per cluster used by the filesystem.
//
//     Size = MAX_FAT_ENTRIES * SECTOR_SIZE * SECTORS_PER_CLUSTER
struct Fat
{
    entries: &'static mut [u32]  // Staticly allocated buffer for the FAT entries.
}



const MAX_FAT_ENTRIES: usize = 65536; // Maximum number of FAT entries we can handle in our buffer.



impl Fat
{
    // Create a new FAT structure by loading the FAT table from the block device and partition. The
    // entire FAT table is cached in RAM in a static buffer.
    pub fn new(block_device: &BlockDevice,
               partition: &LegacyPartition,
               start_sector: usize,
               size_in_sectors: usize) -> Result<Self, &'static str>
    {
        // The static buffer for the FAT entries.
        static mut FAT_BUFFER: [u32; MAX_FAT_ENTRIES] = [0; MAX_FAT_ENTRIES];

        // Get a safe reference to the static buffer. This is safe because we we are executing in a
        // single threaded context and it's up to the containing code to make sure we don't try to
        // construct multiple FAT structures at the same time.
        let buffer = unsafe { &mut FAT_BUFFER[..] };

        // Load the FAT table from the block device into the static buffer and return the loaded
        // table to the caller.
        Self::load_fat_table(block_device, partition, start_sector, size_in_sectors, buffer)?;

        Ok(Fat { entries: unsafe { &mut FAT_BUFFER } })
    }

    // Actually load the File Allocation Table (FAT) from the given block device and partition.
    fn load_fat_table(block_device: &BlockDevice,
                      partition: &LegacyPartition,
                      start_sector: usize,
                      size_in_sectors: usize,
                      buffer: &'static mut [u32]) -> Result<(), &'static str>
    {
        // Allocate a buffer from the sector cache to read the FAT sectors into. We also make sure
        // that the buffer will be freed when we are done with it.
        let (index, sector_buffer) = get_sector_buffer();
        let _defer = Defer::new(|| free_sector_buffer(index));

        // Calculate the starting LBA of the FAT table based on the partition start LBA and the
        // starting sector offset.
        let fat_lba_start = partition.start_lba as usize + start_sector;

        // Keep track of where we are loading the FAT entries.
        let mut buffer_index = 0;

        // Read the FAT sectors from the block device into the buffer.
        for i in 0..size_in_sectors
        {
            // Calculate the LBA of the current sector in the FAT table.
            let lba = (fat_lba_start + i) as u64;

            // Read the sector from the block device.
            block_device.read_sector(lba, sector_buffer)?;

            // Extract the FAT entries from the sector buffer and place them into the FAT buffer.
            for chunk in sector_buffer.chunks_exact(4)
            {
                if buffer_index >= buffer.len()
                {
                    return Err("FAT too large for buffer.");
                }

                // Read the 4 bytes from the sector buffer and convert them to a u32 to store in the
                // FAT buffer.
                buffer[buffer_index] = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                buffer_index += 1;
            }
        }

        Ok(())
    }

    // Look up the next cluster in a given cluster's chain. None is returned if there are no further
    // clusters in the chain.
    //
    // None is also returned if the cluster is invalid.
    pub fn get_next_cluster(&self, cluster: usize) -> Option<usize>
    {
        let cluster = cluster & 0x0FFF_FFFF;

        if cluster >= self.entries.len()
        {
            return None;
        }

        let entry = (self.entries[cluster] & 0x0FFF_FFFF) as usize;

        match entry
        {
            0x0FFF_FFF8..=0x0FFF_FFFF => None,        // End of chain markers.
            0x0FFFFFF7                => None,        // Reserved cluster.
            0                         => None,        // Free cluster.
            _                         => Some(entry)  // Valid cluster.
        }
    }

    // Is the given cluster the end of a chain of clusters?
    pub fn is_end_of_chain(&self, cluster: usize) -> bool
    {
        let next = self.get_next_cluster(cluster);

        next.is_none()
    }
}



// Offsets in the FAT32 header structure. This is not a comprehensive list. We only attempt to
// access the fields we require for iterating the root directory and finding the kernel file in the
// FAT32 filesystem.
const BYTES_PER_SECTOR_OFF:    usize = 0x000b;
const SECTORS_PER_CLUSTER_OFF: usize = 0x000d;
const RESERVED_SECTORS_OFF:    usize = 0x000e;
const NUM_FATS_OFF:            usize = 0x0010;
const FAT_SIZE_32_OFF:         usize = 0x0024;
const ROOT_CLUSTER_OFF:        usize = 0x002c;
const FAT_SIGNATURE_OFF:       usize = 0x01fe;



const BOOT_SIGNATURE: usize = 0xAA55;  // Standard block signature for FAT32 filesystems.



// Represents a FAT32 filesystem as stored in a partition on a block device. This structure contains
// only the necessary information to iterate the root directory and find files in the filesystem.
pub struct Fat32Volume<'a>
{
    pub block_device: &'a BlockDevice,   // The block device containing the FAT32 volume.
    pub partition: &'a LegacyPartition,  // The partition information for the FAT32 volume.
    pub fat: Fat,                        // The FAT table for the FAT32 volume, which maps clusters
                                         // to their next cluster in the chain.
    pub bytes_per_sector: usize,         // The number of bytes per sector in the FAT32 volume.
    pub sectors_per_cluster: usize,      // The number of sectors per cluster in the FAT32 volume.
    pub reserved_sectors: usize,         // The number of reserved sectors in the FAT32 volume.
    pub num_fats: usize,                 // The number of FAT tables in the FAT32 volume.
    pub fat_size_sectors: usize,         // The size of each FAT table in sectors.
    pub root_cluster: usize              // The first cluster of the root directory in the FAT32
                                         // volume.
}



impl<'a> Fat32Volume<'a>
{
    // Initialize and return the representation of the FAT32 filesystem. This function reads the
    // FAT32 boot sector from the first sector of the partition and extracts the necessary fields to
    // construct the FAT32 volume structure.
    pub fn new(block_device: &'a BlockDevice, partition: &'a LegacyPartition) -> FatResult<Self>
    {
        // Get a buffer from the sector cache and make sure that it will be freed when we are done
        // with it. This buffer will be used to read the FAT32 boot sector.
        let (index, mut buffer) = get_sector_buffer();
        let _defer = Defer::new(|| free_sector_buffer(index));

        // Read the first sector of the partition to get the FAT32 boot sector.
        block_device.read_sector(partition.start_lba as u64, &mut buffer)?;

        // Make sure that the boot sector is valid.
        let signature = Self::read_u16(&buffer, FAT_SIGNATURE_OFF)?;

        if signature != BOOT_SIGNATURE
        {
            return Err("Invalid boot signature in FAT32 header.");
        }

        // Read the FAT32 header fields to dig into the filesystem.
        let bytes_per_sector    = Self::read_u16(&buffer, BYTES_PER_SECTOR_OFF)?;
        let sectors_per_cluster = Self::read_u8(&buffer, SECTORS_PER_CLUSTER_OFF)? as usize;
        let reserved_sectors    = Self::read_u16(&buffer, RESERVED_SECTORS_OFF)? as usize;
        let num_fats            = Self::read_u8(&buffer, NUM_FATS_OFF)? as usize;
        let root_cluster        = Self::read_u32(&buffer, ROOT_CLUSTER_OFF)? as usize;
        let fat_size_sectors    = Self::read_u32(&buffer, FAT_SIZE_32_OFF)? as usize;

        // Compute the offset of the first data sector.
        let first_data_sector = reserved_sectors + (num_fats * fat_size_sectors);

        // Validate the sector size.
        if bytes_per_sector != SECTOR_SIZE
        {
            return Err("Invalid bytes per sector in FAT32 header.");
        }

        // Load the file allocation table (FAT) from the block device.
        let fat = Fat::new(block_device, partition, reserved_sectors, fat_size_sectors)?;

        // Construct and return the FAT32 volume structure.
        let volume = Fat32Volume
            {
                block_device,
                partition,
                fat,
                bytes_per_sector,
                sectors_per_cluster,
                reserved_sectors,
                num_fats,
                fat_size_sectors,
                root_cluster
            };

        Ok(volume)
    }

    // Load a sector from a FAT cluster from the filesystem into the provided buffer.
    //
    // Returns an error if the sector could not be loaded or if the cluster or sector is invalid.
    pub fn load_sector(&self,
                       cluster: usize,
                       sector: usize,
                       buffer: &mut SectorBuffer) -> FatResult<()>
    {
        let first_data_sector = self.reserved_sectors + (self.num_fats * self.fat_size_sectors);

        let cluster_lba = first_data_sector + ((cluster - 2) * self.sectors_per_cluster) + sector;
        let absolute_lba = self.partition.start_lba as usize + cluster_lba;

        self.block_device.read_sector(absolute_lba as u64, buffer)?;

        Ok(())
    }

    // Read a u8 value from the FAT32 volume header. An error is returned if the offset is out of
    // bounds of the sector buffer.
    fn read_u8(buffer: &SectorBuffer, offset: usize) -> Result<usize, &'static str>
    {
        if offset < SECTOR_SIZE
        {
            Ok(buffer[offset] as usize)
        }
        else
        {
            Err("Offset out of bounds for sector buffer.")
        }
    }

    // Read a u16 value from the FAT32 volume header. An error is returned if the offset is out of
    // bounds of the sector buffer.
    fn read_u16(buffer: &SectorBuffer, offset: usize) -> Result<usize, &'static str>
    {
        if offset + 1 < SECTOR_SIZE
        {
            Ok(u16::from_le_bytes([buffer[offset], buffer[offset + 1]]) as usize)
        }
        else
        {
            Err("Offset out of bounds for sector buffer.")
        }
    }

    // Read a u32 value from the FAT32 volume header. An error is returned if the offset is out of
    // bounds of the sector buffer.
    fn read_u32(buffer: &SectorBuffer, offset: usize) -> Result<u32, &'static str>
    {
        if offset + 3 < SECTOR_SIZE
        {
            Ok(u32::from_le_bytes([buffer[offset],
                                   buffer[offset + 1],
                                   buffer[offset + 2],
                                   buffer[offset + 3]]))
        }
        else
        {
            Err("Offset out of bounds for sector buffer.")
        }
    }
}



// Interface for streaming through the data in a file in the FAT32 filesystem. This is also used as
// the basis for reading directories in the filesystem.
//
// Directories are just special files that contain entries for each file in the directory.
pub struct FileStream<'a>
{
    fat_volume: &'a Fat32Volume<'a>,  // The FAT32 volume we are reading from.
    start_cluster: usize,             // The starting cluster of the file we are reading.
    size: usize,                      // The size of the file in bytes we are reading.
    current_cluster: usize,           // The current cluster we are reading from.
    current_sector: usize,            // The current sector within the cluster we are reading.
    current_byte: usize,              // The byte offset into the current sector of the file.
    absolute_byte: usize,             // The absolute byte offset into the file we are reading.
    buffer: &'a mut SectorBuffer,     // The buffer for the current sector we are reading.
    buffer_index: usize               // The index of the sector buffer in the sector cache.
}



impl<'a> FileStream<'a>
{
    // Create an initialize a new file stream for reading a file or directory data from the FAT32
    // filesystem.
    //
    // Given a FAT32 volume, the starting cluster of the file, and the size of the file in bytes,
    pub fn new(fat_volume: &'a Fat32Volume<'a>,
               start_cluster: usize,
               size: usize) -> FatResult<Self>
    {
        // Allocate a sector buffer from the sector cache. We will load sectors from the filesystem
        // into this buffer as we stream through the file.
        let (index, buffer) = get_sector_buffer();

        let mut fs = FileStream
            {
                fat_volume,
                start_cluster,
                size,
                current_cluster: start_cluster,
                current_sector: 0,
                current_byte: 0,
                absolute_byte: 0,
                buffer,
                buffer_index: index
            };

        // Check to see if the file has any data in it. If it does, we will load the first sector
        // into the buffer so that we are ready to read from it.
        if fs.size != 0
        {
            fs.load_current_sector()?;
        }

        Ok(fs)
    }

    // Move the file cursor back to the beginning of the file.
    pub fn reset(&mut self) -> FatResult<()>
    {
        // Reset the file stream to the beginning of the file.
        self.current_cluster = self.start_cluster;
        self.current_sector = 0;
        self.current_byte = 0;
        self.absolute_byte = 0;

        // Load the first sector again.
        self.load_current_sector()?;

        Ok(())
    }

    // Is the file cursor at the end of the file?
    pub fn is_eof(&self) -> bool
    {
        // Check if we have read all the bytes in the file.
        self.absolute_byte >= self.size
    }

    // Read a single byte from the file stream, advancing the cursor.
    pub fn read_u8(&mut self) -> FatResult<u8>
    {
        let mut value: u8 = 0;

        self.read_data(&mut value)?;
        Ok(value)
    }

    // Read a u16 value from the file stream, advancing the cursor.
    pub fn read_u16(&mut self) -> FatResult<u16>
    {
        let mut value: u16 = 0;

        self.read_data(&mut value)?;
        Ok(value)
    }

    // Read a u32 value from the file stream, advancing the cursor.
    pub fn read_u32(&mut self) -> FatResult<u32>
    {
        let mut value: u32 = 0;

        self.read_data(&mut value)?;
        Ok(value)
    }

    // Read a u64 value from the file stream, advancing the cursor.
    pub fn read_u64(&mut self) -> FatResult<u64>
    {
        let mut value: u64 = 0;

        self.read_data(&mut value)?;
        Ok(value)
    }

    // Read a fixed-size data structure from the file stream, advancing the cursor. Make sure that
    // the data structure padding is correct and fits with the expected data layout from disk.
    pub fn read_data<T>(&mut self, data: &mut T) -> FatResult<()>
        where
            T: Sized
    {
        let raw_bytes = unsafe
            {
                // Get a mutable pointer to the data structure. Then create a slice of the raw bytes
                // of the data structure.
                let ptr = data as *mut T;
                from_raw_parts_mut(ptr as *mut u8, size_of::<T>())
            };

        self.read_bytes(raw_bytes);

        Ok(())
    }

    // Read an untyped collection of bytes from the file stream, advancing the cursor the number of
    // bytes in the slice. If the entire slice can not be filled, an error is returned.
    pub fn read_bytes(&mut self, buffer: &mut [u8]) -> FatResult<()>
    {
        // Attempt to read the specified number of bytes into the buffer.
        for index in 0..buffer.len()
        {
            // Read the next byte from the file stream.
            match self.next_byte()?
            {
                Some(byte) =>
                {
                    // Store the byte in the buffer.
                    buffer[index] = byte;
                },

                None =>
                {
                    // We have reached the end of the file before filling the buffer.
                    return Err("End of file reached before filling buffer.");
                }
            }

            // Advance to the next sector if necessary.
            if self.is_end_of_sector()
            {
                // We have reached the end of the current sector, so we need to advance to the next
                // sector in the file.
                if !self.next_sector()?
                {
                    // We have reached the end of the file chain, so we can not read any more sectors.
                    return Err("End of file reached.");
                }
            }
        }

        // We've successfully read the entire buffer.
        Ok(())
    }

    // Is the cursor at the end of the current sector?
    fn is_end_of_sector(&self) -> bool
    {
        // Check if we have reached the end of the current sector.
        self.current_byte >= SECTOR_SIZE
    }

    // Read the next byte from the file stream, returning None if we are at the end of the file.
    fn next_byte(&mut self) -> FatResult<Option<u8>>
    {
        // Have we hit the end the file?
        if self.is_eof()
        {
            return Ok(None);
        }

        // Check if we have reached the end of the current sector.
        if self.is_end_of_sector()
        {
            if !self.next_sector()?
            {
                return Ok(None);
            }
        }

        // Read the next byte from the current sector buffer.
        let byte = self.buffer[self.current_byte];

        // Advance the file cursor to the next byte.
        self.absolute_byte += 1;
        self.current_byte += 1;

        Ok(Some(byte))
    }

    // Advance the file cursor to the next sector in the file. This will advance to the next cluster
    // in the file if necessary. If there are no more sectors in the file we return false.
    //
    // If the sector can not be read, we return an error.
    fn next_sector(&mut self) -> FatResult<bool>
    {
        // Advance to the next sector in the current cluster.
        self.current_sector += 1;

        // Check if we have reached the end of the current cluster.
        if self.current_sector >= self.fat_volume.sectors_per_cluster
        {
            // We have reached the end of the current cluster, so we need to move to the next
            // cluster in the file chain.
            self.current_sector = 0;

            // Get the next cluster in the file chain.
            match self.fat_volume.fat.get_next_cluster(self.current_cluster)
            {
                Some(next_cluster) =>
                {
                    // We have a next cluster, so we can continue reading.
                    self.current_cluster = next_cluster;
                },

                None =>
                {
                    // We have reached the end of the file chain, so we can not read any more
                    // sectors.
                    return Ok(false)
                }
            }

            // We have moved to the next cluster so load the now current sector into our buffer.
            self.load_current_sector()?;
        }

        Ok(true)
    }

    // Load the current sector into the sector buffer. It is the responsibility of the caller to
    // advance the file cursor to the next sector after loading.
    fn load_current_sector(&mut self) -> FatResult<()>
    {
        // Reset the byte offset into the current sector.
        self.current_byte = 0;

        // Check if we are trying to read outside of the partition. We also make sure we're not
        // trying to read one of the reserved clusters in the FAT32 filesystem.
        if    self.current_cluster < 2
           || self.current_cluster >= self.fat_volume.fat.entries.len()
        {
            return Err("Attempt to read outside of the partition.");
        }

        // Make sure that the sector number is valid.
        if self.current_sector >= self.fat_volume.sectors_per_cluster
        {
            return Err("Attempt to read outside of the current cluster.");
        }

        // Load the current sector from the FAT32 volume.
        self.fat_volume.load_sector(self.current_cluster, self.current_sector, self.buffer)?;

        Ok(())
    }
}



// Make sure we free the sector buffer when the file stream is dropped.
impl<'a> Drop for FileStream<'a>
{
    fn drop(&mut self)
    {
        free_sector_buffer(self.buffer_index);
    }
}



// Represents a directory entry in the FAT32 filesystem.
#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirectoryEntry
{
    pub name: [u8; 11],          // The name of the file (8.3 format).
    pub attributes: u8,          // File attributes (read-only, hidden, system, volume label,
                                 //   directory, archive).
    pub reserved: u8,            // Reserved byte.
    pub creation_time_tenth: u8, // Creation time (tenths of a second).
    pub creation_time: u16,      // Creation time (hours and minutes).
    pub creation_date: u16,      // Creation date (year, month, day).
    pub last_access_date: u16,   // Last access date.
    pub first_cluster_high: u16, // High word of the first cluster number.
    pub last_write_time: u16,    // Last write time (hours and minutes).
    pub last_write_date: u16,    // Last write date (year, month, day).
    pub first_cluster_low: u16,  // Low word of the first cluster number.
    pub file_size: u32           // Size of the file in bytes.
}



const DIRECTORY_ENTRY_SIZE: usize = 32;  // Size of a directory entry in bytes (FAT32).



const _: () =
    {
        assert!(size_of::<DirectoryEntry>() == DIRECTORY_ENTRY_SIZE);
    };



impl DirectoryEntry
{
    pub fn new(file: &mut FileStream) -> FatResult<Self>
    {
        let mut new_entry = Self::zeroed();

        file.read_data(&mut new_entry)?;
        Ok(new_entry)
    }

    pub fn zeroed() -> Self
    {
        DirectoryEntry
            {
                name: [0; 11],
                attributes: 0,
                reserved: 0,
                creation_time_tenth: 0,
                creation_time: 0,
                creation_date: 0,
                last_access_date: 0,
                first_cluster_high: 0,
                last_write_time: 0,
                last_write_date: 0,
                first_cluster_low: 0,
                file_size: 0
            }
    }

    pub fn first_cluster(&self) -> usize
    {
        // Combine the high and low words of the first cluster number to get the full cluster number.
        ((self.first_cluster_high as usize) << 16) | (self.first_cluster_low as usize)
    }

    pub fn is_file(&self) -> bool
    {
        // Check if the entry is a file (not a directory).
        (self.attributes & 0x10) == 0
    }

    pub fn is_end_of_directory(&self) -> bool
    {
        // Check if the entry is an end-of-directory marker.
        self.name == [0; 11] && self.file_size == 0
    }

    pub fn is_deleted(&self) -> bool
    {
        // Check if the entry is marked as deleted.
        self.name[0] == 0xE5
    }
}



// Iterator for iterating through the entries in a directory in the FAT32 filesystem.
pub struct DirectoryIterator<'a>
{
    file_stream: FileStream<'a>,  // The file stream for reading the directory entries.
    base_cluster: usize           // The base cluster of the directory we are iterating over.
}



impl<'a> DirectoryIterator<'a>
{
    // Create a new directory iterator from the given cluster address in the FAT32 filesystem.
    // Internally we create a file stream for reading the directory entries from the directory file.
    pub fn new(fat_volume: &'a Fat32Volume<'a>, base_cluster: usize) -> FatResult<Self>
    {
        // Compute the size of the directory and create a new file stream for the given directory.
        let directory_size = Self::calculate_directory_size(fat_volume, base_cluster)?;
        let file_stream = FileStream::new(fat_volume, base_cluster, directory_size)?;

        Ok(DirectoryIterator { file_stream, base_cluster })
    }

    // Calculate the size of the directory file by iterating through the clusters in the directory
    // chain. This is needed by the root directory because there is no directory entry for the root
    // directory to tell us it's size.
    fn calculate_directory_size(fat_volume: &Fat32Volume, base_cluster: usize) -> FatResult<usize>
    {
        let mut cluster = base_cluster;
        let mut total_size = 0;
        let cluster_size = fat_volume.sectors_per_cluster * SECTOR_SIZE;

        loop
        {
            total_size += cluster_size;

            if let Some(next_cluster) = fat_volume.fat.get_next_cluster(cluster)
            {
                // Move to the next cluster in the chain.
                cluster = next_cluster;
            }
            else
            {
                // We have reached the end of the directory entries.
                break;
            }
        }

        Ok(total_size)
    }

    // Given a function, iterate through the directory entries in the directory file. The callback
    // function is called once per directory entry. If the callback returns false, the iteration is
    // stopped. Otherwise the iteration continues until the end of the directory is hit.
    pub fn iterate<Func>(&mut self, mut callback: Func) -> FatResult<()>
        where
            Func: FnMut(&DirectoryEntry) -> bool
    {
        // Make sure we're starting at the beginning of the directory entry list.
        self.file_stream.reset()?;

        loop
        {
            // Load the next directory entry from the file stream.
            let entry = DirectoryEntry::new(&mut self.file_stream)?;

            // Check if we have reached the end of the directory entries.
            if entry.is_end_of_directory()
            {
                // We have reached the end of the directory entries.
                break;
            }

            // Skip deleted entries.
            if entry.is_deleted()
            {
                continue;
            }

            // Call the callback with the current directory entry.
            if !callback(&entry)
            {
                // The callback returned false, so we stop iterating.
                break;
            }
        }

        Ok(())
    }
}
