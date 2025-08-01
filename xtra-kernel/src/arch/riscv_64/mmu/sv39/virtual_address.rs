
// Definition of a virtual address as defined under the sv39 page table format specification.

use core::ops::Deref;

use crate::arch::mmu::{ PAGE_SIZE, sv39::page_table::PAGE_TABLE_SIZE };



/// These bits are reserved for future use and must be set to zero.
const PTA_RESERVED: u64
//          6            5           4            3           2            1           0
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_1111_1111_1111_1111_1111_1111_1000_0000_0000_0000_0000_0000_0000_0000_0000_0000;

/// Physical Address section 2.
const PTA_VPN_2: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0111_1111_1100_0000_0000_0000_0000_0000_0000_0000;

/// Physical Address section 1.
const PTA_VPN_1: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0011_1111_1110_0000_0000_0000_0000_0000;

/// Physical Address section 0.
const PTA_VPN_0: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0001_1111_1111_0000_0000_0000;

/// Page offset.
const PTA_OFFSET: u64
//          6            5           4            3           2            1
//       3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210 9876 5432 1098 7654 3210
    = 0b_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_0000_1111_1111_1111;



/// Representation of a virtual address in the SV39 page table format.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VirtualAddress(usize);



impl VirtualAddress
{
    /// Create a new virtual address from the given raw address.
    pub fn new_from_address<T>(address: *const T) -> Self
    {
        Self::new(address as usize)
    }

    /// Create a new virtual address from the given raw address.
    pub fn new(address: usize) -> Self
    {
        assert!((address & (PTA_RESERVED as usize)) == 0,
                "A virtual address must not have reserved bits set. Address: {:#x} \
                Reserved bits: {:#x}",
                address,
                PTA_RESERVED as usize);

        Self(address)
    }

    /// Get the page table entry address for this virtual address.
    /// Index 0 is the leaf (lowest) level (VPN[0]), 2 is the root (VPN[2])
    pub fn get_vpn(&self, index: usize) -> usize
    {
        match index
        {
            0 => (self.0 & (PTA_VPN_0 as usize)) >> 12,
            1 => (self.0 & (PTA_VPN_1 as usize)) >> 21,
            2 => (self.0 & (PTA_VPN_2 as usize)) >> 30,
            _ => panic!("Invalid virtual address VPN index: {}", index)
        }
    }

    /// Set the page table entry address for this virtual address.
    /// Index 0 is the leaf (lowest) level (VPN[0]), 2 is the root (VPN[2]).
    pub fn set_vpn(&mut self, index: usize, vpn: usize)
    {
        assert!(vpn < PAGE_TABLE_SIZE,
                "Virtual Page Number (VPN) must fit in the VPN section of the virtual address. \
                Got: {}, but max is: {}",
                vpn,
                PAGE_TABLE_SIZE - 1);

        match index
        {
            0 => self.0 = (self.0 & !(PTA_VPN_0 as usize)) | ((vpn << 12) & (PTA_VPN_0 as usize)),
            1 => self.0 = (self.0 & !(PTA_VPN_1 as usize)) | ((vpn << 21) & (PTA_VPN_1 as usize)),
            2 => self.0 = (self.0 & !(PTA_VPN_2 as usize)) | ((vpn << 30) & (PTA_VPN_2 as usize)),
            _ => panic!("Invalid virtual address VPN index: {}", index)
        }
    }

    /// Get the offset within the page being addressed by this virtual address.
    pub fn get_offset(&self) -> usize
    {
        self.0 & (PTA_OFFSET as usize)
    }

    /// Set the offset within the page being addressed by this virtual address.
    pub fn set_offset(&mut self, offset: usize)
    {
        assert!(offset < PAGE_SIZE,
                "Offset must be less than the page size. Got: {}, but max is: {}",
                offset,
                PAGE_SIZE);

        self.0 = (self.0 & !(PTA_OFFSET as usize)) | (offset & (PTA_OFFSET as usize));
    }
}



/// Allow for easy dereferencing of the virtual address to a usize, which is useful for passing
/// the address to functions that expect a raw pointer or address.
impl Deref for VirtualAddress
{
    type Target = usize;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}
