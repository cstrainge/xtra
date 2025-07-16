
// Provide higher level abstractions for the RISC-V Control Status Registers, CSRs. These are
// special registers that control various aspects of the RISC-V architecture, such as interrupts,
// performance counters, and other system-level and user-level features.

use core::arch::asm;



// Generic function for reading a Control Status Register, (CSR) in the RISC-V architecture. No
// validation is done on the CSR number. It's up to the caller to ensure we're requesting a valid
// CSR.
macro_rules! read_csr
{
    ($csr:expr) =>
        {{
            let value: u64;

            unsafe
            {
                asm!
                (
                    "csrr {0}, {1}",

                    out(reg) value,
                    const $csr,

                    options(nomem, nostack, preserves_flags)
                );
            }

            value
        }};
}



// Generic function for writing a value to a Control Status Register. No validation is done on the
// CSR number or the value being written. It is up to the caller to ensure we're writing to an
// existing CSR and that the value is valid for that CSR.
#[inline(always)]
fn write_csr(csr: usize, value: u64)
{
    unsafe
    {
        asm!
        (
            "csrw {0}, {1}",

            in(reg) value,
            in(reg) csr,

            options(nomem, nostack, preserves_flags)
        );
    }
}



// List of CSRs that are available in the RISC-V architecture.


// Machine Information Registers.
const CSR_MVENDORID:     usize = 0xf11;  // Vendor ID.
const CSR_MARCHID:       usize = 0xf12;  // Architecture ID.
const CSR_MIMPID:        usize = 0xf13;  // Implementation ID.
const CSR_MHARTID:       usize = 0xf14;  // Hardware thread ID.
const CSR_MCONFIGPTR:    usize = 0xf15;  // Pointer to configuration data structure.


// Machine Memory Protection Registers.
const CSR_PMPCFG00:      usize = 0x3A0;  // Physical memory protection configuration.
const CSR_PMPCFG14:      usize = 0x3ae;  // Physical memory protection configuration.

const CSR_PMPADDR00:     usize = 0x3b0;  // Physical memory protection address register.
const CSR_PMPADDR63:     usize = 0x3ef;  // Physical memory protection address register.


// Machine counters/timers.
const CSR_MCYCLE:        usize = 0xb00;  // Machine cycle counter.
const CSR_MINSTRET:      usize = 0xb02;  // Machine instructions-retired counter.


// Register collection counts.
const CSR_PMPCFG_COUNT:  usize = (CSR_PMPCFG14 - CSR_PMPCFG00) / 2;
const CSR_PMPADDR_COUNT: usize = (CSR_PMPADDR63 - CSR_PMPADDR00) + 1;



// ---- Machine Information Registers -------------------------------------------------------------

pub fn read_mvendorid() -> u64
{
    read_csr!(CSR_MVENDORID)
}



pub fn read_marchid() -> u64
{
    read_csr!(CSR_MARCHID)
}



pub fn read_mimpid() -> u64
{
    read_csr!(CSR_MIMPID)
}



pub fn read_mhartid() -> u64
{
    read_csr!(CSR_MHARTID)
}



pub fn read_mconfigptr() -> u64
{
    read_csr!(CSR_MCONFIGPTR)
}



// ---- Machine Memory Protection Registers --------------------------------------------------------

const PMP_CFG_R:     u64 = 0b_0000_0001;  // Read access.
const PMP_CFG_W:     u64 = 0b_0000_0010;  // Write access.
const PMP_CFG_X:     u64 = 0b_0000_0100;  // Execute access.
const PMP_CFG_TOR:   u64 = 0b_0000_1000;  // Top-of-range mode.
const PMP_CFG_NAPOT: u64 = 0b_0001_0000;  // Next address power of two mode.
const PMP_CFG_L:     u64 = 0b_1000_0000;  // Locked configuration.



/*pub fn read_pmpcfg(index: usize) -> u64
{
    if index > CSR_PMPCFG_COUNT
    {
        panic!("Invalid PMP configuration index: {}", index);
    }

    read_csr!(CSR_PMPCFG00 + (index * 2))
}



pub fn write_pmpcfg(index: usize, value: u64)
{
    if index > CSR_PMPCFG_COUNT
    {
        panic!("Invalid PMP configuration index: {}", index);
    }

    write_csr(CSR_PMPCFG00 + (index * 2), value);
}



pub fn read_pmpaddr(index: usize) -> u64
{
    if index > CSR_PMPADDR_COUNT
    {
        panic!("Invalid PMP address index: {}", index);
    }

    read_csr!(CSR_PMPADDR00 + index)
}



pub fn write_pmpaddr(index: usize, value: u64)
{
    if index > CSR_PMPADDR_COUNT
    {
        panic!("Invalid PMP address index: {}", index);
    }

    write_csr(CSR_PMPADDR00 + index, value);
}*/



// ---- Machine Counters/Timers --------------------------------------------------------------------

pub fn read_cycle_counter() -> u64
{
    read_csr!(CSR_MCYCLE)
}



pub fn read_instruction_counter() -> u64
{
    read_csr!(CSR_MINSTRET)
}
