
const BOOT_SIGNATURE:          u16   = 0xAA55;  // Boot signature for MBR.

const PARTITION_TYPE_EMPTY:    u8    = 0x00;    // Empty partition type.
const PARTITION_TYPE_FAT32:    u8    = 0x0C;    // FAT32 partition type.
const PARTITION_TYPE_EXTENDED: u8    = 0x05;    // Extended partition type.

pub const MBR_SIZE:            usize = 512;     // Size of the Master Boot Record (MBR) on disk.
pub const MBR_CODE_SIZE:       usize = 446;     // Size of the boot code in the MBR.
pub const MBR_PARTITION_COUNT: usize = 4;       // Number of partition entries in the MBR.
pub const MBR_PARTITION_SIZE:  usize = 16;      // Size of each partition entry in the MBR.


pub type MbrBytes          = [u8; MBR_SIZE];
pub type MbrPartitions     = [LegacyPartition; MBR_PARTITION_COUNT];
pub type MbrCode           = [u8; MBR_CODE_SIZE];
pub type MbrPartitionBytes = [u8; MBR_PARTITION_SIZE];



#[derive(Clone, Copy)]
pub enum PartitionStatus
{
    Inactive,
    Bootable,
    Unknown(u8)
}



#[derive(Clone, Copy)]
pub enum PartitionType
{
    Empty,
    Fat32,
    Extended,
    Unknown(u8)
}



#[derive(Clone, Copy)]
pub struct LegacyPartition
{
    pub status: PartitionStatus,        // Partition status (active or inactive).
    pub start_chs: [u8; 3],             // Start CHS (Cylinder/Head/Sector).
    pub partition_type: PartitionType,  // Partition type identifier.
    pub end_chs: [u8; 3],               // End CHS (Cylinder/Head/Sector).
    pub start_lba: u32,                 // Starting sector number (LBA).
    pub size_in_sectors: u32            // Size in sectors.
}



impl LegacyPartition
{
    fn new(bytes: &MbrPartitionBytes) -> Self
    {
        let status = bytes[0];
        let start_chs = [bytes[1], bytes[2], bytes[3]];
        let partition_type = bytes[4];
        let end_chs = [bytes[5], bytes[6], bytes[7]];
        let start_lba = u32::from_le_bytes([bytes[8],
                                            bytes[9],
                                            bytes[10],
                                            bytes[11]]);
        let size_in_sectors = u32::from_le_bytes([bytes[12],
                                                  bytes[13],
                                                  bytes[14],
                                                  bytes[15]]);

        LegacyPartition
            {
                status: Self::partition_status(status),
                start_chs,
                partition_type: Self::partition_type(partition_type),
                end_chs,
                start_lba,
                size_in_sectors
            }
    }

    fn partition_status(status: u8) -> PartitionStatus
    {
        match status
        {
            0x00  => PartitionStatus::Inactive,
            0x80  => PartitionStatus::Bootable,
            other => PartitionStatus::Unknown(other)
        }
    }

    fn partition_type(partition_type: u8) -> PartitionType
    {
        match partition_type
        {
            PARTITION_TYPE_EMPTY    => PartitionType::Empty,
            PARTITION_TYPE_FAT32    => PartitionType::Fat32,
            PARTITION_TYPE_EXTENDED => PartitionType::Extended,
            other                   => PartitionType::Unknown(other)
        }
    }

    pub fn is_bootable(&self) -> bool
    {
           matches!(self.status, PartitionStatus::Bootable)
        && matches!(self.partition_type, PartitionType::Fat32)
    }
}



#[derive(Clone, Copy)]
pub struct MasterBootRecord
{
    boot_code: MbrCode,         // Boot code (executable x86 code).
    partitions: MbrPartitions,  // Partition entries.
    boot_signature: u16         // Boot signature (0x55AA).
}



impl MasterBootRecord
{
    pub fn new(bytes: &MbrBytes) -> Self
    {
        let boot_code = bytes[0..446].try_into().unwrap();
        let partitions =
            [
                LegacyPartition::new(&bytes[446..462].try_into().unwrap()),
                LegacyPartition::new(&bytes[462..478].try_into().unwrap()),
                LegacyPartition::new(&bytes[478..494].try_into().unwrap()),
                LegacyPartition::new(&bytes[494..510].try_into().unwrap())
            ];
        let boot_signature = u16::from_le_bytes([bytes[510], bytes[511]]);

        MasterBootRecord
            {
                boot_code,
                partitions,
                boot_signature
            }
    }

    pub fn is_valid(&self) -> bool
    {
        self.boot_signature == BOOT_SIGNATURE
    }

    pub fn partitions(&self) -> &MbrPartitions
    {
        &self.partitions
    }
}
