
// Core module for handling the hardware abstraction layer (HAL) for the various CPU subsystems.



/// All the RISC-V 64 specific code for the kernel.
#[cfg(target_arch = "riscv64")]
mod riscv_64;



// Export the architecture specific code based on the platform we are compiling for. This will
// allow us to use the same code for both RISC-V and other architectures in the future.
#[cfg(target_arch = "riscv64")]
pub use riscv_64::*;



use crate::arch::csr::read_mhartid;



/// Get the index of the current core.
pub fn get_core_index() -> usize
{
    read_mhartid() as usize
}
