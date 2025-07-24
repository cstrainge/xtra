
// The memory structure of the kernel is defined here. This code works with the linker script to
// define the memory layout of the kernel in RAM.

use core::fmt::{ self, Display, Formatter };



// Represents a section of memory in the kernel layout,
pub struct SectionLayout
{
    pub start: usize,  // Start address of the section.
    pub end: usize,    // End address of the section.
    pub size: usize    // Size of the section in bytes.
}



// The layout of the kernel in RAM, this includes the kernel range, text, rodata, data, and bss
// sections.
pub struct KernelMemoryLayout
{
    pub kernel: SectionLayout,  // The entire kernel range in RAM.
    pub text: SectionLayout,    // The memory used by the kernel code.
    pub rodata: SectionLayout,  // The read-only data section of the kernel.
    pub data: SectionLayout,    // The initialized data section of the kernel.
    pub bss: SectionLayout,     // The uninitialized data section of the kernel.
    pub stack: SectionLayout    // The stacks for each hart in the system.
}



impl KernelMemoryLayout
{
    // Create a new instance of the kernel memory layout, this will read the kernel sections from
    // the linker script and return a new `KernelMemoryLayout` instance.
    pub fn new() -> Self
    {
        extern "C"
        {
            static _kernel_start: u8;
            static _kernel_end: u8;
            static _text_start: u8;
            static _text_end: u8;
            static _rodata_start: u8;
            static _rodata_end: u8;
            static _data_start: u8;
            static _data_end: u8;
            static _bss_start: u8;
            static _bss_end: u8;
            static _stack_start: u8;
            static _stack_end: u8;
        }

        let kernel_start = unsafe { &_kernel_start as *const u8 as usize };
        let kernel_end = unsafe { &_kernel_end as *const u8 as usize };

        let text_start = unsafe { &_text_start as *const u8 as usize };
        let text_end = unsafe { &_text_end as *const u8 as usize };
        let rodata_start = unsafe { &_rodata_start as *const u8 as usize };
        let rodata_end = unsafe { &_rodata_end as *const u8 as usize };
        let data_start = unsafe { &_data_start as *const u8 as usize };
        let data_end = unsafe { &_data_end as *const u8 as usize };
        let bss_start = unsafe { &_bss_start as *const u8 as usize };
        let bss_end = unsafe { &_bss_end as *const u8 as usize };
        let stack_start = unsafe { &_stack_start as *const u8 as usize };
        let stack_end = unsafe { &_stack_end as *const u8 as usize };

        KernelMemoryLayout
            {
                kernel:
                    SectionLayout
                    {
                        start: kernel_start,
                        end: kernel_end,
                        size: kernel_end - kernel_start
                    },

                text:
                    SectionLayout
                    {
                        start: text_start,
                        end: text_end,
                        size: text_end - text_start
                    },

                rodata:
                    SectionLayout
                    {
                        start: rodata_start,
                        end: rodata_end,
                        size: rodata_end - rodata_start
                    },

                data:
                    SectionLayout
                    {
                        start: data_start,
                        end: data_end,
                        size: data_end - data_start
                    },

                bss:
                    SectionLayout
                    {
                        start: bss_start,
                        end: bss_end,
                        size: bss_end - bss_start
                    },

                stack:
                    SectionLayout
                    {
                        start: stack_start,
                        end: stack_end,
                        size: stack_end - stack_start
                    }
            }
    }
}



// Print the kernel memory layout in a human-readable format for diagnostics purposes.
impl Display for KernelMemoryLayout
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result
    {
        writeln!(f, "Kernel Memory Layout:")?;
        write!(f, "  Range:             0x{:08x} - 0x{:08x}: ",
                 self.kernel.start,
                 self.kernel.end)?;
        write_size!(f, self.kernel.size)?;
        writeln!(f)?;

        writeln!(f, "  Sections:")?;
        write!(f, "    .text:           0x{:08x} - 0x{:08x}: ",
                 self.text.start,
                 self.text.end)?;
        write_size!(f, self.text.size)?;
        writeln!(f)?;

        write!(f, "    .rodata:         0x{:08x} - 0x{:08x}: ",
                 self.rodata.start,
                 self.rodata.end)?;
        write_size!(f, self.rodata.size)?;
        writeln!(f)?;

        write!(f, "    .data:           0x{:08x} - 0x{:08x}: ",
                 self.data.start,
                 self.data.end)?;
        write_size!(f, self.data.size)?;
        writeln!(f)?;

        write!(f, "    .bss:            0x{:08x} - 0x{:08x}: ",
                 self.bss.start,
                 self.bss.end)?;
        write_size!(f, self.bss.size)?;
        writeln!(f)?;

        write!(f, "    .stack:          0x{:08x} - 0x{:08x}: ",
                 self.stack.start,
                 self.stack.end)?;
        write_size!(f, self.stack.size)?;
        writeln!(f)?;

        Ok(())
    }
}
