


const BOOT_SIGNATURE: u16 = 0xAA55;  // Boot signature for MBR.


const PARTITION_TYPE_EMPTY: u8 = 0x00; // Empty partition type.
const PARTITION_TYPE_FAT32: u8 = 0x0C; // FAT32 partition type.
const PARTITION_TYPE_EXTENDED: u8 = 0x05; // Extended partition type.
const PARTITION_TYPE_LINUX: u8 = 0x83; // Linux partition type.



pub struct Partition
{
    status: u8,           // 0x80 = bootable, 0x00 = inactive.
    start_chs: [u8; 3],   // Start CHS (Cylinder/Head/Sector).
    partition_type: u8,   // Partition type identifier.
    end_chs: [u8; 3],     // End CHS (Cylinder/Head/Sector).
    start_lba: u32,       // Starting sector number (LBA).
    size_in_sectors: u32  // Size in sectors.
}



impl Partition
{
    fn new(bytes: &[u8; 16]) -> Self
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

        Partition {
            status,
            start_chs,
            partition_type,
            end_chs,
            start_lba,
            size_in_sectors
        }
    }
}



pub struct MasterBootRecord
{
    boot_code: [u8; 446],               // Boot code (executable x86 code).
    partition_entries: [Partition; 4],  // Partition entries.
    boot_signature: u16                 // Boot signature (0x55AA).
}



impl MasterBootRecord
{
    pub fn new(bytes: &[u8; 512]) -> Self
    {
        let boot_code = bytes[0..446].try_into().unwrap();
        let partition_entries =
            [
                Partition::new(&bytes[446..462].try_into().unwrap()),
                Partition::new(&bytes[462..478].try_into().unwrap()),
                Partition::new(&bytes[478..494].try_into().unwrap()),
                Partition::new(&bytes[494..510].try_into().unwrap())
            ];
        let boot_signature = u16::from_le_bytes([bytes[510], bytes[511]]);

        MasterBootRecord
            {
                boot_code,
                partition_entries,
                boot_signature
            }
    }

    pub fn is_valid(&self) -> bool
    {
        self.boot_signature == BOOT_SIGNATURE
    }
}
