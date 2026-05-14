
// The definition of the mount table used by the Kernel to bring in the base file system(s) and to
// know where to mount them in the filesystem tree. If nothing is mounted in / then the Kernel will
// fail to boot the operating system.

use core::{ clone::Clone,
            cmp::{ Eq, PartialEq },
            default::Default,
            marker::Copy,
            prelude::rust_2024::derive };



/// The maximum number of entries in the mount table.
pub const XTRA_MAX_MOUNT_TABLE_ENTRIES: usize = 16;



/// The maximum length of the mount point string, this is the path in the filesystem where the
/// device will be mounted.
pub const XTRA_MAX_MOUNT_POINT_STRING_LENGTH: usize = 64;



/// The types of filesystems that the Xtra Kernel supports.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum XtraFilesystemType
{
    /// No filesystem specified.
    None,

    /// The FAT32 filesystem, this is a simple filesystem that is widely supported and is used for
    /// all kinds of devices.
    Fat32,

    /// The Ext2 filesystem, this is a more complex filesystem that is used for larger storage
    /// devices and is the base for important filesystem features like permission support.
    Ext2
}



/// The representation of a single entry in the mount table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct XtraMountTableEntry
{
    /// Where in the filesystem tree should this device be mounted?
    pub mount_point: [u8; XTRA_MAX_MOUNT_POINT_STRING_LENGTH],

    /// Index of the drive to mount from.
    pub device: u8,

    /// Partition of the drive to mount from.
    pub partition: u8,

    /// The type of filesystem to expect on this device partition.
    pub filesystem_type: XtraFilesystemType
}



impl XtraMountTableEntry
{
    /// Creates a new unassigned mount table entry.
    pub const fn new() -> XtraMountTableEntry
    {
        XtraMountTableEntry
            {
                mount_point: [0; XTRA_MAX_MOUNT_POINT_STRING_LENGTH],
                device: u8::MAX,
                partition: u8::MAX,
                filesystem_type: XtraFilesystemType::None
            }
    }
}



/// The default unassigned mount table entry.
impl Default for XtraMountTableEntry
{
    fn default() -> XtraMountTableEntry
    {
        XtraMountTableEntry::new()
    }
}



/// The representation of the mount table.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct XtraMountTable
{
    /// The number of entries actually used in the mount table.
    pub num_entries: usize,

    /// The entries in the mount table.
    pub entries: [XtraMountTableEntry; XTRA_MAX_MOUNT_TABLE_ENTRIES]
}



impl XtraMountTable
{
    /// Creates a new empty mount table.
    pub const fn new() -> XtraMountTable
    {
        XtraMountTable
            {
                num_entries: 0,
                entries: [XtraMountTableEntry::new(); XTRA_MAX_MOUNT_TABLE_ENTRIES]
            }
    }
}



/// The default empty mount table.
impl Default for XtraMountTable
{
    fn default() -> XtraMountTable
    {
        XtraMountTable::new()
    }
}
