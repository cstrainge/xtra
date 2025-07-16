
use core::{ arch::asm,
            mem::{ align_of, size_of },
            ptr::{ addr_of_mut, read_volatile, write_volatile },
            sync::atomic::{ fence, Ordering::{ Acquire, Release, SeqCst } },
            time } ;



#[derive(Clone, Copy)]
pub struct MmioRegister<const OFFSET: usize>(*mut u32);



impl<const OFFSET: usize> MmioRegister<OFFSET>
{
    pub fn new(address: usize) -> Self
    {
        MmioRegister((address + OFFSET) as *mut u32)
    }

    #[inline(always)]
    pub unsafe fn read(&self) -> u32
    {
        read_volatile(self.0)
    }

    #[inline(always)]
    pub unsafe fn write(&self, value: u32)
    {
        write_volatile(self.0, value);
    }
}



// VirtIO MMIO device register offsets (all u32, little-endian)
pub const MAGIC_VALUE:            usize = 0x000;  // 0x74726976 ("virt".)
pub const VERSION:                usize = 0x004;  // Device version (1 or 2.)
pub const DEVICE_ID:              usize = 0x008;  // Device type (2 = block, 1 = net, etc.)
pub const VENDOR_ID:              usize = 0x00C;  // Vendor ID ("QEMU" = 0x554D4551.)

pub const DEVICE_FEATURES:        usize = 0x010; // Device feature bits [31:0].
pub const DEVICE_FEATURES_SEL:    usize = 0x014; // Selects feature bits [63:32].
pub const DRIVER_FEATURES:        usize = 0x020; // Driver feature bits [31:0].
pub const DRIVER_FEATURES_SEL:    usize = 0x024; // Selects feature bits [63:32].

pub const QUEUE_SEL:              usize = 0x030; // Select which queue to access.
pub const QUEUE_NUM_MAX:          usize = 0x034; // Max size of selected queue.
pub const QUEUE_NUM:              usize = 0x038; // Queue size (<= max).
// pub const QUEUE_ALIGN:            usize = 0x03C; // (Legacy only, ignore for modern devices).
// pub const QUEUE_PFN:              usize = 0x040; // (Legacy only, ignore for modern devices).

pub const QUEUE_READY:            usize = 0x044; // Set to 1 to activate queue (modern.)
pub const QUEUE_NOTIFY:           usize = 0x050; // Notify device that queue has work.

pub const INTERRUPT_STATUS:       usize = 0x060; // IRQ status bits.
pub const INTERRUPT_ACK:          usize = 0x064; // Acknowledge IRQ.

pub const STATUS:                 usize = 0x070; // Device/driver status bits.

// Modern "split virt-queue" addressing (added in VirtIO 1.0+):
pub const QUEUE_DESC_LOW:         usize = 0x080; // [31:0] physical addr of descriptor table.
pub const QUEUE_DESC_HIGH:        usize = 0x084; // [63:32] physical addr of descriptor table.

pub const QUEUE_AVAIL_LOW:        usize = 0x090; // [31:0] addr of avail ring.
pub const QUEUE_AVAIL_HIGH:       usize = 0x094; // [63:32] addr of avail ring.

pub const QUEUE_USED_LOW:         usize = 0x0A0; // [31:0] addr of used ring.
pub const QUEUE_USED_HIGH:        usize = 0x0A4; // [63:32] addr of used ring.

// Optional: config generation (rarely used, sometimes omitted.)
pub const CONFIG_GENERATION:      usize = 0x0FC;  // Incremented on config change.

// Device-specific config (starts at 0x100 or higher)
pub const DEVICE_CONFIG:          usize = 0x100;



// Block device register offsets (for VirtIO 1.0+):
pub const TOTAL_SECTOR_COUNT_LOW:  usize = 0x100;
pub const TOTAL_SECTOR_COUNT_HIGH: usize = 0x104;
pub const MAX_SEGMENT_SIZE:        usize = 0x108;
pub const MAX_SEGMENT_COUNT:       usize = 0x10c;
pub const CYLINDER_COUNT:          usize = 0x110;
pub const HEAD_COUNT:              usize = 0x112;
pub const SECTOR_COUNT:            usize = 0x113;
pub const BLOCK_LENGTH:            usize = 0x114;



#[derive(Clone, Copy)]
pub struct MmioDevice
{
    magic: MmioRegister<MAGIC_VALUE>,
    version: MmioRegister<VERSION>,
    device_id: MmioRegister<DEVICE_ID>,
    vendor_id: MmioRegister<VENDOR_ID>,

    device_features: MmioRegister<DEVICE_FEATURES>,
    device_features_select: MmioRegister<DEVICE_FEATURES_SEL>,

    driver_features: MmioRegister<DRIVER_FEATURES>,
    driver_features_select: MmioRegister<DRIVER_FEATURES_SEL>,

    pub queue_select: MmioRegister<QUEUE_SEL>,
    queue_num_max: MmioRegister<QUEUE_NUM_MAX>,
    queue_num: MmioRegister<QUEUE_NUM>,
    // queue_align: MmioRegister<QUEUE_ALIGN>,
    // queue_pfn: MmioRegister<QUEUE_PFN>,

    pub queue_ready: MmioRegister<QUEUE_READY>,
    queue_notify: MmioRegister<QUEUE_NOTIFY>,

    interrupt_status: MmioRegister<INTERRUPT_STATUS>,
    interrupt_ack: MmioRegister<INTERRUPT_ACK>,

    status: MmioRegister<STATUS>,

    pub queue_desc_low: MmioRegister<QUEUE_DESC_LOW>,
    pub queue_desc_high: MmioRegister<QUEUE_DESC_HIGH>,

    queue_avail_low: MmioRegister<QUEUE_AVAIL_LOW>,
    queue_avail_high: MmioRegister<QUEUE_AVAIL_HIGH>,

    queue_used_low: MmioRegister<QUEUE_USED_LOW>,
    queue_used_high: MmioRegister<QUEUE_USED_HIGH>,

    // config_generation: MmioRegister<CONFIG_GENERATION>,

    // Block device specific registers.
    total_sector_count_low: MmioRegister<TOTAL_SECTOR_COUNT_LOW>,
    total_sector_count_high: MmioRegister<TOTAL_SECTOR_COUNT_HIGH>,
    max_segment_size: MmioRegister<MAX_SEGMENT_SIZE>,
    max_segment_count: MmioRegister<MAX_SEGMENT_COUNT>,
    cylinder_count: MmioRegister<CYLINDER_COUNT>,
    head_count: MmioRegister<HEAD_COUNT>,
    sector_count: MmioRegister<SECTOR_COUNT>,
    block_length: MmioRegister<BLOCK_LENGTH>
}



impl MmioDevice
{
    pub fn new(base_address: usize) -> Self
    {
        MmioDevice
            {
                magic: MmioRegister::new(base_address),
                version: MmioRegister::new(base_address),
                device_id: MmioRegister::new(base_address),
                vendor_id: MmioRegister::new(base_address),

                device_features: MmioRegister::new(base_address),
                device_features_select: MmioRegister::new(base_address),
                driver_features: MmioRegister::new(base_address),
                driver_features_select: MmioRegister::new(base_address),

                queue_select: MmioRegister::new(base_address),
                queue_num_max: MmioRegister::new(base_address),
                queue_num: MmioRegister::new(base_address),
                // queue_align: MmioRegister::new(base_address),
                // queue_pfn: MmioRegister::new(base_address),

                queue_ready: MmioRegister::new(base_address),
                queue_notify: MmioRegister::new(base_address),

                interrupt_status: MmioRegister::new(base_address),
                interrupt_ack: MmioRegister::new(base_address),

                status: MmioRegister::new(base_address),

                queue_desc_low: MmioRegister::new(base_address),
                queue_desc_high: MmioRegister::new(base_address),

                queue_avail_low: MmioRegister::new(base_address),
                queue_avail_high: MmioRegister::new(base_address),

                queue_used_low: MmioRegister::new(base_address),
                queue_used_high: MmioRegister::new(base_address),

                total_sector_count_low: MmioRegister::new(base_address),
                total_sector_count_high: MmioRegister::new(base_address),
                max_segment_size: MmioRegister::new(base_address),
                max_segment_count: MmioRegister::new(base_address),
                cylinder_count: MmioRegister::new(base_address),
                head_count: MmioRegister::new(base_address),
                sector_count: MmioRegister::new(base_address),
                block_length: MmioRegister::new(base_address)
            }
    }


    #[inline(always)]
    pub fn magic(&self) -> u32
    {
        unsafe { self.magic.read() }
    }

    #[inline(always)]
    pub fn version(&self) -> u32
    {
        unsafe { self.version.read() }
    }

    #[inline(always)]
    pub fn device_id(&self) -> u32
    {
        unsafe { self.device_id.read() }
    }

    #[inline(always)]
    pub fn vendor_id(&self) -> u32
    {
        unsafe { self.vendor_id.read() }
    }


    #[inline(always)]
    pub fn device_features_partial(&self, select: u32) -> u32
    {
        unsafe { self.device_features_select.write(select) };
        fence(SeqCst);
        unsafe { self.device_features.read() }
    }

    #[inline(always)]
    pub fn device_features(&self) -> u64
    {
        let low = self.device_features_partial(0) as u64;
        let high = self.device_features_partial(1) as u64;

        (high << 32) | low
    }

    #[inline(always)]
    pub fn set_driver_features_partial(&self, select: u32, features: u32)
    {
        unsafe { self.driver_features_select.write(select) };
        fence(SeqCst);
        unsafe { self.driver_features.write(features) };
    }

    #[inline(always)]
    pub fn set_driver_features(&self, features: u64)
    {
        let low  = (features & 0xFFFF_FFFF) as u32;
        let high = (features >> 32) as u32;

        self.set_driver_features_partial(0, low);
        self.set_driver_features_partial(1, high);
    }

    #[inline(always)]
    pub fn set_queue_select(&self, select: u32)
    {
        unsafe { self.queue_select.write(select) };
    }

    #[inline(always)]
    pub fn queue_num_max(&self) -> u32
    {
        unsafe { self.queue_num_max.read() }
    }

    #[inline(always)]
    pub fn set_queue_num(&self, num: u32)
    {
        unsafe { self.queue_num.write(num) };
    }


    #[inline(always)]
    pub fn queue_ready(&self) -> bool
    {
        unsafe { self.queue_ready.read() != 0 }
    }

    #[inline(always)]
    pub fn set_queue_ready(&self, ready: bool)
    {
        unsafe { self.queue_ready.write(if ready { 1 } else { 0 }) };
    }

    #[inline(always)]
    pub fn notify_queue(&self, queue: u32)
    {
        fence(Release);
        unsafe { self.queue_notify.write(queue) };
    }


    #[inline(always)]
    pub fn interrupt_status(&self) -> u32
    {
        unsafe { self.interrupt_status.read() }
    }

    #[inline(always)]
    pub fn interrupt_ack(&self, status: u32)
    {
        unsafe { self.interrupt_ack.write(status) };
    }


    #[inline(always)]
    pub fn status(&self) -> u32
    {
        unsafe { self.status.read() }
    }

    #[inline(always)]
    pub fn set_status(&self, value: u32)
    {
        unsafe { self.status.write(value) };
    }


    #[inline(always)]
    pub fn add_status(&self, value: u32)
    {
        unsafe { self.status.write(self.status() | value) };
    }


    #[inline(always)]
    pub fn set_queue_descriptors<T>(&self, address: *const T)
    {
        let address = address as usize;

        unsafe { self.queue_desc_low.write((address & 0xFFFF_FFFF) as u32) };
        unsafe { self.queue_desc_high.write((address >> 32) as u32) };
    }

    #[inline(always)]
    pub fn set_queue_available<T>(&self, address: *const T)
    {
        let address = address as usize;

        unsafe { self.queue_avail_low.write((address & 0xFFFF_FFFF) as u32) };
        unsafe { self.queue_avail_high.write((address >> 32) as u32) };
    }

    #[inline(always)]
    pub fn set_queue_used<T>(&self, address: *const T)
    {
        let address = address as usize;

        unsafe { self.queue_used_low.write((address & 0xFFFF_FFFF) as u32) };
        unsafe { self.queue_used_high.write((address >> 32) as u32) };
    }

    #[inline(always)]
    pub fn total_sector_count(&self) -> u64
    {
        let low = unsafe { self.total_sector_count_low.read() } as u64;
        let high = unsafe { self.total_sector_count_high.read() } as u64;

        (high << 32) | low
    }

    // Block device specific methods.

    #[inline(always)]
    pub fn max_segment_size(&self) -> u32
    {
        unsafe { self.max_segment_size.read() }
    }

    #[inline(always)]
    pub fn max_segment_count(&self) -> u32
    {
        unsafe { self.max_segment_count.read() }
    }

    #[inline(always)]
    pub fn cylinder_count(&self) -> u16
    {
        unsafe { self.cylinder_count.read() as u16 }
    }

    #[inline(always)]
    pub fn head_count(&self) -> u8
    {
        unsafe { self.head_count.read() as u8 }
    }

    #[inline(always)]
    pub fn sector_count(&self) -> u8
    {
        unsafe { self.sector_count.read() as u8 }
    }

    #[inline(always)]
    pub fn block_length(&self) -> u32
    {
        unsafe { self.block_length.read() }
    }
}



pub const SECTOR_SIZE: usize = 512;



pub type Sector = [u8; SECTOR_SIZE];



pub type IoResult<T> = Result<T, &'static str>;



pub const VIRTIO_MMIO_MAGIC:           u32   = 0x74726976;

pub const VIRTIO_BLOCK_DEVICE_ID:      u32   = 2;

// Status bits
pub const VIRTIO_CONFIG_S_ACKNOWLEDGE: u32   = 0x01;
pub const VIRTIO_CONFIG_S_DRIVER:      u32   = 0x02;
pub const VIRTIO_CONFIG_S_DRIVER_OK:   u32   = 0x04;
pub const VIRTIO_CONFIG_S_FEATURES_OK: u32   = 0x08;
pub const VIRTIO_CONFIG_S_FAILED:      u32   = 0x80;

// Generic feature bits
pub const VIRTIO_F_VERSION_1:          u64   = 1 << 32;

// Block request flags.
pub const VIRTIO_BLK_T_IN:             u32   = 0;
pub const VIRTIO_BLK_T_OUT:            u32   = 1;

// Device feature bits.
pub const VIRTIO_BLK_F_RO:             u32   =  5;
pub const VIRTIO_BLK_F_SCSI:           u32   =  7;
pub const VIRTIO_BLK_F_CONFIG_WCE:     u32   = 11;
pub const VIRTIO_BLK_F_MQ:             u32   = 12;
pub const VIRTIO_F_ANY_LAYOUT:         u32   = 27;
pub const VIRTIO_RING_F_INDIRECT_DESC: u32   = 28;
pub const VIRTIO_RING_F_EVENT_IDX:     u32   = 29;

pub const VIRTQ_AVAIL_F_NO_INTERRUPT:  u16   = 1;
pub const VIRTQ_USED_F_NO_NOTIFY:      u16   = 1;

// VirtIO descriptor flags
pub const VIRTQ_DESC_F_NEXT: u16 = 1;
pub const VIRTQ_DESC_F_WRITE: u16 = 2;

// The size of our VirtIO block device queues.
pub const QUEUE_SIZE:                  usize = 8;
pub const PAGE_SIZE:                   usize = 4096;



#[repr(C, align(16))]
#[derive(Clone, Copy)]
struct Descriptor
{
    address: u64,
    length: u32,
    flags: u16,
    next: u16
}



impl Descriptor
{
    pub const fn zeroed() -> Self
    {
        Descriptor
            {
                address: 0,
                length: 0,
                flags: 0,
                next: 0
            }
    }
}



#[repr(C, align(2))]
struct AvailableRing
{
    flags: u16,
    index: u16,
    ring: [u16; QUEUE_SIZE],
    unused: u16
}



impl AvailableRing
{
    pub const fn zeroed() -> Self
    {
        AvailableRing
            {
                flags: 0,
                index: 0,
                ring: [0; QUEUE_SIZE],
                unused: 0
            }
    }
}



#[repr(C, align(4))]
#[derive(Clone, Copy)]
struct UsedItem
{
    id: u32,
    length: u32
}



impl UsedItem
{
    pub const fn zeroed() -> Self
    {
        UsedItem { id: 0, length: 0 }
    }
}



#[repr(C, align(4))]
struct UsedRing
{
    flags: u16,
    index: u16,
    ring: [UsedItem; QUEUE_SIZE],
    unused: u16
}



impl UsedRing
{
    pub const fn zeroed() -> Self
    {
        UsedRing
            {
                flags: 0,
                index: 0,
                ring: [UsedItem::zeroed(); QUEUE_SIZE],
                unused: 0
            }
    }
}



#[repr(C)]
struct BlockRequest
{
    request_type: u32,  // Request type (read or write)
    reserved: u32,      // Reserved for future use
    sector: u64         // Sector number to read/write
}



impl BlockRequest
{
    pub fn new(request_type: u32, sector: u64) -> Self
    {
        BlockRequest
            {
                request_type,
                reserved: 0,
                sector
            }
    }

    pub fn zeroed() -> Self
    {
        BlockRequest
            {
                request_type: 0,
                reserved: 0,
                sector: 0
            }
    }
}


#[repr(align(4096))]
struct AlignedDescriptors(pub [Descriptor; QUEUE_SIZE]);

impl AlignedDescriptors
{
    pub const fn zeroed() -> Self
    {
        AlignedDescriptors([Descriptor::zeroed(); QUEUE_SIZE])
    }
}

#[repr(align(4096))]
struct AlignedAvailableRing(pub AvailableRing);

impl AlignedAvailableRing
{
    pub const fn zeroed() -> Self
    {
        AlignedAvailableRing(AvailableRing::zeroed())
    }
}

#[repr(align(4096))]
struct AlignedUsedRing(pub UsedRing);

impl AlignedUsedRing
{
    pub const fn zeroed() -> Self
    {
        AlignedUsedRing(UsedRing::zeroed())
    }
}



static mut DESCRIPTORS: AlignedDescriptors = AlignedDescriptors::zeroed();
static mut AVAILABLE_RING: AlignedAvailableRing = AlignedAvailableRing::zeroed();
static mut USED: AlignedUsedRing = AlignedUsedRing::zeroed();
static mut READ_STATUS: u8 = 0xff;



// Make sure that the device visible data structures are the correct size and alignment as per the
// VirtIO specification.
const _: () =
    {
        assert!(size_of::<Descriptor>()            == 16);
        assert!(align_of::<Descriptor>()           == 16);

        assert!(size_of::<AvailableRing>()         == 6 + 2 * QUEUE_SIZE);
        assert!(align_of::<AvailableRing>()        == 2);

        assert!(size_of::<UsedItem>()              == 8);
        assert!(align_of::<UsedItem>()             == 4);

        assert!(size_of::<UsedRing>()              == ((6 + 8 * QUEUE_SIZE) + 3) & !3);
        assert!(align_of::<UsedRing>()             == 4);

        assert!(align_of::<AlignedDescriptors>()   == 4096);
        assert!(align_of::<AlignedAvailableRing>() == 4096);
        assert!(align_of::<AlignedUsedRing>()      == 4096);
    };



// Represents a VirtIO block device.  This structure will handle all the low level communication
// with the VirtIO block device using MMIO (Memory-Mapped I/O) registers.
pub struct VirtIoBlockDevice
{
    mmio: MmioDevice  // The MMIO register set for communicating with the VirtIO block device.
}


impl VirtIoBlockDevice
{
    pub fn new(base_address: usize) -> Self
    {
        VirtIoBlockDevice
            {
                mmio: MmioDevice::new(base_address)
            }
    }

    pub fn initialize(&mut self) -> IoResult<()>
    {
        let uart = crate::uart::Uart::new(0x1000_0000);

        // Make sure that this is a valid VirtIO block device.
        if !self.is_block_device()
        {
            return Err("Not a valid VirtIO block device.");
        }

        // Reset the device.
        self.mmio.set_status(0);

        // Acknowledge the device.
        self.mmio.set_status(VIRTIO_CONFIG_S_ACKNOWLEDGE);

        // Tell the device that we are a driver.
        self.mmio.add_status(VIRTIO_CONFIG_S_DRIVER);

        // Get the device features.
        let mut features = self.mmio.device_features();

        features &= !(1 << VIRTIO_BLK_F_RO);
        features &= !(1 << VIRTIO_BLK_F_SCSI);
        features &= !(1 << VIRTIO_BLK_F_CONFIG_WCE);
        features &= !(1 << VIRTIO_BLK_F_MQ);
        features &= !(1 << VIRTIO_F_ANY_LAYOUT);
        features &= !(1 << VIRTIO_RING_F_EVENT_IDX);
        features &= !(1 << VIRTIO_RING_F_INDIRECT_DESC);

        // Set the supported driver features.
        self.mmio.set_driver_features(features);

        // Notify the device that we are ready to use the features, confirm that the device is ok.
        self.mmio.add_status(VIRTIO_CONFIG_S_FEATURES_OK);

        if self.mmio.status() & VIRTIO_CONFIG_S_FEATURES_OK == 0
        {
            self.mmio.add_status(VIRTIO_CONFIG_S_FAILED);
            return Err("feature negotiation failed");
        }

        // Initialize the device queue 0.
        self.mmio.set_queue_select(0);

        if self.mmio.queue_ready()
        {
            return Err("Queue 0 should not be ready.");
        }

        // Configure the queue size.
        let max = self.mmio.queue_num_max();

        if max == 0
        {
            return Err("VirtIO block device has no queue.");
        }

        if max < QUEUE_SIZE as u32
        {
            return Err("VirtIO block device queue size is too small.");
        }

        self.mmio.set_queue_num(QUEUE_SIZE as u32);

        // Set the pointers to the queue descriptors, available ring, and used ring.
        #[allow(static_mut_refs)]
        unsafe
        {
            self.mmio.set_queue_descriptors(DESCRIPTORS.0.as_ptr());
            self.mmio.set_queue_available(addr_of_mut!(AVAILABLE_RING.0));
            self.mmio.set_queue_used(addr_of_mut!(USED.0));
        }

        // Make sure to disable interrupts for the available and used rings as we are not using
        // them in the bootloader.
        unsafe
        {
            write_volatile(addr_of_mut!(AVAILABLE_RING.0.flags), VIRTQ_AVAIL_F_NO_INTERRUPT);
            write_volatile(addr_of_mut!(USED.0.flags), VIRTQ_USED_F_NO_NOTIFY);

            // Clear any pending interrupts
            let int_status = self.mmio.interrupt_status();

            if int_status != 0
            {
                self.mmio.interrupt_ack(int_status);
            }
        }

        // Enable the queue.
        fence(Release);
        self.mmio.set_queue_ready(true);

        // Notify the device that we are ready to use the queue.
        self.mmio.add_status(VIRTIO_CONFIG_S_DRIVER_OK);

        // Check if the queue is ready.
        if !self.mmio.queue_ready()
        {
            return Err("VirtIO block device queue is not ready.");
        }

        uart.put_str("Block device information:\n");
        uart.put_str("  Total sectors:     ");
        uart.put_int(self.mmio.total_sector_count() as usize);
        uart.put_str("\n");

        uart.put_str("  Max segment size:  ");
        uart.put_int(self.mmio.max_segment_size() as usize);
        uart.put_str("\n");

        uart.put_str("  Max segment count: ");
        uart.put_int(self.mmio.max_segment_count() as usize);
        uart.put_str("\n");

        uart.put_str("  Cylinder count:    ");
        uart.put_int(self.mmio.cylinder_count() as usize);
        uart.put_str("\n");

        uart.put_str("  Head count:        ");
        uart.put_int(self.mmio.head_count() as usize);
        uart.put_str("\n");

        uart.put_str("  Sector count:      ");
        uart.put_int(self.mmio.sector_count() as usize);
        uart.put_str("\n");

        uart.put_str("  Block length:      ");
        uart.put_int(self.mmio.block_length() as usize);
        uart.put_str("\n");

        Ok(())
    }

    pub fn read_sector(&self, sector: u64, buffer: &mut Sector) -> IoResult<()>
    {
        let request = BlockRequest::new(VIRTIO_BLK_T_IN, sector);

        unsafe
        {
            READ_STATUS = 0xff;

            DESCRIPTORS.0[0] = Descriptor
                {
                    address: &request as *const BlockRequest as u64,
                    length: size_of::<BlockRequest>() as u32,
                    flags: VIRTQ_DESC_F_NEXT,
                    next: 1
                };

            DESCRIPTORS.0[1] = Descriptor
                {
                    address: buffer.as_mut_ptr() as u64,
                    length: SECTOR_SIZE as u32,
                    flags: VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT,
                    next: 2
                };

            DESCRIPTORS.0[2] = Descriptor
                {
                    address: &raw mut READ_STATUS as *mut u8 as u64,
                    length: size_of::<u8>() as u32,
                    flags: VIRTQ_DESC_F_WRITE,
                    next: 0
                };

            let available_index = AVAILABLE_RING.0.index as usize % QUEUE_SIZE;
            AVAILABLE_RING.0.ring[available_index] = 0; // Descriptor index 0
            //AVAILABLE_RING.index += 1;

            fence(SeqCst);
            AVAILABLE_RING.0.index = AVAILABLE_RING.0.index.wrapping_add(1);
            fence(SeqCst);
        }

        self.mmio.notify_queue(0);

        // Wait for the device to process the request.
        let starting_used_index = unsafe { USED.0.index };
        let mut timeout = 10_000_000;
        let mut last_read = unsafe { USED.0.index };

        while    last_read == starting_used_index
              && timeout > 0
        {
            timeout -= 1;
            unsafe { asm!("nop") };

            last_read = unsafe { USED.0.index };
            fence(Acquire);
        }

        if timeout == 0
        {
            return Err("Timeout waiting for VirtIO block device response.");
        }

        match unsafe { READ_STATUS }
        {
            0 => Ok(()),
            1 => Err("VirtIO block device error: Invalid request."),
            2 => Err("VirtIO block device error: Device not ready."),
            3 => Err("VirtIO block device error: IO error."),
            _ => Err("Unknown VirtIO block device error.")
        }
    }

    // Validate that the device is a valid VirtIO block device.
    pub fn is_block_device(&self) -> bool
    {
           self.mmio.magic() == VIRTIO_MMIO_MAGIC
        && matches!(self.mmio.version(), 1 | 2)
        && self.mmio.device_id() == VIRTIO_BLOCK_DEVICE_ID
    }
}
