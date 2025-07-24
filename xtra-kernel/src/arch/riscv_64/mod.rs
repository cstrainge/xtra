
// The base of the RISC-V 64-bit architecture module. All of the architecture specific code is
// included here and in it's sub-modules.
//
// This module contains all the architecture specific code for the RISC-V 64-bit architecture.



/// All of the RISC-V CSR register access functions.
pub mod csr;

/// The hardware level MMU support for RISC-V 64-bit.
pub mod mmu;



use crate::{ arch::csr::{ read_marchid, read_mhartid, read_mimpid, read_mvendorid },
             print, println };



/// Print out information about the running CPU architecture.
pub fn print_cpu_info()
{
    let vendor_id = read_mvendorid();
    let arch_id   = read_marchid();
    let imp_id    = read_mimpid();
    let hart_id   = read_mhartid();

    println!("RISC-V CPU Information:");
    println!("  Vendor ID:         0x{:x}", vendor_id);
    println!("  Arch ID:           0x{:x}", arch_id);
    println!("  Implementation ID: 0x{:x}", imp_id);
    println!("  Hart ID:           {:02}",  hart_id);
    println!();
}
