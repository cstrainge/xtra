
// Our elf file parsing and loading code. We also perform validation of the ELF file to ensure it is
// compatible with the architecture we are running on.

use core::{ mem::transmute, slice::from_raw_parts_mut };

use crate::{ fat32::FileStream, uart::Uart };



// The 64-bit ELF header structure as defined in the ELF specification.
#[repr(C, packed)]
pub struct Elf64Header
{
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16
}



// Elf header constants...
const ELF_MAGIC:   [u8; 4] = [0x7f, b'E', b'L', b'F'];

const ELF_VERSION: u32     = 1;    // Original version of the ELF specification.
const EM_RISCV:    u16     = 0xf3; // EM_RISCV: RISC-V architecture.
const ET_EXEC:     u16     = 2;    // ET_EXEC: Executable file.
const EI_CLASS_64: u8      = 2;    // EI_CLASS: 2 for 64-bit.
const EI_DATA:     u8      = 1;    // EI_DATA: 1 for little-endian.



// Ensure the size of the ELF header is correct. This is a compile-time assertion.
const _ : () =
    {
        assert!(size_of::<Elf64Header>() == 64);
    };



impl Elf64Header
{
    // Read the ELF header from the file stream and return a new Elf64Header instance.
    pub fn new(file_stream: &mut FileStream) -> Result<Self, &'static str>
    {
        let mut header = Elf64Header::zeroed();

        file_stream.read_data(&mut header)?;
        Ok(header)
    }

    // Create a new blank ELF header with all fields zeroed.
    pub fn zeroed() -> Self
    {
        Elf64Header
            {
                e_ident: [0; 16],
                e_type: 0,
                e_machine: 0,
                e_version: 0,
                e_entry: 0,
                e_phoff: 0,
                e_shoff: 0,
                e_flags: 0,
                e_ehsize: 0,
                e_phentsize: 0,
                e_phnum: 0,
                e_shentsize: 0,
                e_shnum: 0,
                e_shstrndx: 0
            }
    }

    // Check to see if the magic number is valid.
    pub fn is_valid(&self) -> bool
    {
        self.e_ident[0..4] == ELF_MAGIC
    }

    // Check if the version of the elf file is supported by this loader.
    pub fn version_supported(&self) -> bool
    {
        self.e_version == ELF_VERSION
    }

    // Does the elf file represent an executable?
    pub fn is_executable(&self) -> bool
    {
        self.e_type == ET_EXEC
    }

    // Was the elf file compiled for RISC-V architecture?
    pub fn is_riscv(&self) -> bool
    {
        self.e_machine == EM_RISCV
    }

    // Is the elf file a 64-bit executable?
    pub fn is_64_bit(&self) -> bool
    {
        self.e_ident[4] == EI_CLASS_64
    }

    // Was the elf file compiled in little-endian format?
    pub fn is_little_endian(&self) -> bool
    {
        self.e_ident[5] == EI_DATA
    }
}



#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Elf64ProgramHeader
{
    pub p_type: u32,     // Segment type.
    pub p_flags: u32,    // Segment flags.
    pub p_offset: u64,   // Offset in file.
    pub p_vaddr: u64,    // Virtual address in memory.
    pub p_paddr: u64,    // Physical address (ignore for user programs).
    pub p_filesz: u64,   // Size in file.
    pub p_memsz: u64,    // Size in memory.
    pub p_align: u64     // Alignment.
}



// Program header constants...
const PT_NULL:    u32 = 0;    // Null segment, unused.
const PT_LOAD:    u32 = 1;    // Loadable segment.
const PT_DYNAMIC: u32 = 2;    // Dynamic linking information.
const PT_INTERP:  u32 = 3;    // Interpreter information.
const PT_NOTE:    u32 = 4;    // Auxiliary information.

// Access flags.
const PF_X:       u32 = 0x1;  // Executable.
const PF_W:       u32 = 0x2;  // Writable.
const PF_R:       u32 = 0x4;  // Readable.




// As before we make sure that the size of the program header is correct.
const _: () =
    {
        assert!(size_of::<Elf64ProgramHeader>() == 56);
    };



const MAX_PROGRAM_HEADERS: usize = 8;  // Maximum number of program headers we support in a single
                                       //  ELF file.



impl Elf64ProgramHeader
{
    // Read the program header from the file stream and return a new Elf64ProgramHeader instance.
    pub fn new(file_stream: &mut FileStream) -> Result<Self, &'static str>
    {
        let mut header = Elf64ProgramHeader::zeroed();

        file_stream.read_data(&mut header)?;
        Ok(header)
    }

    // Create a new blank program header with all fields zeroed.
    pub fn zeroed() -> Self
    {
        Elf64ProgramHeader
            {
                p_type: 0,
                p_flags: 0,
                p_offset: 0,
                p_vaddr: 0,
                p_paddr: 0,
                p_filesz: 0,
                p_memsz: 0,
                p_align: 0
            }
    }

    // Check if the segment is a loadable segment.
    pub fn is_loadable(&self) -> bool
    {
        self.p_type == PT_LOAD
    }
}



// Define the function to execute the kernel. It's expected to take the hart ID and device tree
// pointer as arguments and never return.
type KernelEntryPoint = extern "C" fn(hart_id: usize, device_tree_ptr: *const u8) -> !;



// Make sure the ELF file heder is valid and compiled for the architecture we are running on.
fn validate_elf_header(header: &Elf64Header) -> Result<(), &'static str>
{
    if !header.is_valid()
    {
        return Err("Invalid ELF header magic value.");
    }

    if !header.version_supported()
    {
        return Err("Unsupported ELF version.");
    }

    if !header.is_executable()
    {
        return Err("ELF file is not an executable.");
    }

    if !header.is_riscv()
    {
        return Err("ELF file is not compiled for RISC-V architecture.");
    }

    if !header.is_64_bit()
    {
        return Err("ELF file is not a 64-bit executable.");
    }

    if !header.is_little_endian()
    {
        return Err("ELF file is not in little-endian format.");
    }

    Ok(())
}



fn load_segment(program_header: &Elf64ProgramHeader,
                file_stream: &mut FileStream) -> Result<(), &'static str>
{
    let destination_address = program_header.p_vaddr as *mut u8;
    let position = file_stream.tell();

    // Seek to the segment's offset in the file.
    file_stream.seek(program_header.p_offset as usize)?;

    unsafe
    {
        let destination_slice = from_raw_parts_mut(destination_address,
                                                   program_header.p_filesz as usize);

        file_stream.read_bytes(destination_slice)?;

        // Zero out any remaining memory (p_memsz > p_filesz for BSS sections.)
        if program_header.p_memsz > program_header.p_filesz
        {
            let zero_start = destination_address.offset(program_header.p_filesz as isize);
            let zero_size  = (program_header.p_memsz - program_header.p_filesz) as usize;
            let zero_slice = from_raw_parts_mut(zero_start, zero_size);

            zero_slice.fill(0);
        }
    }

    // Restore the old file stream position for the next header.
    file_stream.seek(position)?;

    Ok(())
}



// Stream all loadable segments from the ELF file to the specified load address in memory.
fn stream_kernel_segments(uart: &Uart,
                          load_address: *const u8,
                          elf_header: &Elf64Header,
                          file_stream: &mut FileStream) -> Result<(), &'static str>
{
    // Seek to the start of the program header table.
    file_stream.seek(elf_header.e_phoff as usize)?;

    let mut program_headers = [Elf64ProgramHeader::zeroed(); MAX_PROGRAM_HEADERS];

    // Read the program headers from the file stream.
    if elf_header.e_phnum as usize > MAX_PROGRAM_HEADERS
    {
        return Err("Too many program headers in ELF file.");
    }

    uart.put_str("Loading kernel header segments from offset: ");
    uart.put_hex(elf_header.e_phoff as usize, true);
    uart.put_str("\n");

    for index in 0..elf_header.e_phnum as usize
    {
        let position = file_stream.tell();
        program_headers[index] = Elf64ProgramHeader::new(file_stream)?;

        uart.put_str("  Processing program header: ");
        uart.put_int(index as usize);
        uart.put_str(" @ ");
        uart.put_hex(position as usize, true);
        uart.put_str("\n");

        uart.put_str("    Type:             ");
        uart.put_hex(program_headers[index].p_type as usize, true);
        uart.put_str("\n");

        uart.put_str("    Flags:            ");
        uart.put_hex(program_headers[index].p_flags as usize, true);
        uart.put_str("\n");

        uart.put_str("    Offset:           ");
        uart.put_hex(program_headers[index].p_offset as usize, true);
        uart.put_str("\n");

        uart.put_str("    Virtual Address:  ");
        uart.put_hex(program_headers[index].p_vaddr as usize, true);
        uart.put_str("\n");

        uart.put_str("    Physical Address: ");
        uart.put_hex(program_headers[index].p_paddr as usize, true);
        uart.put_str("\n");

        uart.put_str("    File Size:        ");
        uart.put_int(program_headers[index].p_filesz as usize);
        uart.put_str("\n");

        uart.put_str("    Memory Size:      ");
        uart.put_hex(program_headers[index].p_memsz as usize, true);
        uart.put_str("\n");

        uart.put_str("    Alignment:        ");
        uart.put_hex(program_headers[index].p_align as usize, true);
        uart.put_str("\n");
    }

    // Process each program header.
    for index in 0..elf_header.e_phnum
    {
        let program_header = program_headers[index as usize];

        if program_header.is_loadable()
        {
            load_segment(&program_header, file_stream)?;
        }
    }

    Ok(())
}



// Load the kernel from the file stream and execute it at the given memory address. We will pass the
// hart ID and device tree pointer as arguments to the kernel.
//
// In the future we may want to pass additional arguments like command line arguments or other
// configuration data.
pub fn execute_kernel(uart: &Uart,
                      load_address: *const u8,
                      hart_id: usize,
                      device_tree_ptr: *const u8,
                      file_stream: &mut FileStream) -> Result<(), &'static str>
{
    // Read and validate the ELF header from the file stream.
    let elf_header = Elf64Header::new(file_stream)?;

    validate_elf_header(&elf_header)?;

    uart.put_str("Loading kernel to memory address: ");
    uart.put_hex(load_address as usize, true);
    uart.put_str("\n");

    uart.put_str("  Kernel entry point: ");
    uart.put_hex(elf_header.e_entry as usize, true);
    uart.put_str("\n");

    uart.put_str("  Program header offset: ");
    uart.put_hex(elf_header.e_phoff as usize, true);
    uart.put_str("\n");

    uart.put_str("  Program header count: ");
    uart.put_int(elf_header.e_phnum as usize);
    uart.put_str("\n");

    // Load the kernel into memory at the specified load address.
    stream_kernel_segments(uart, load_address, &elf_header, file_stream)?;

    // Get the entry point address from the ELF header.
    let entry_point = elf_header.e_entry;

    // Get the kernel entry point function pointer and finally, call it.
    unsafe
    {
        let kernel_entry: KernelEntryPoint = transmute(entry_point);

        kernel_entry(hart_id, device_tree_ptr);
    }

    Ok(())
}
