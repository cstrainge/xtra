
/*
 * Bring in the memory locations that are shared between the kernel and the bootloader.
 */

INCLUDE shared_locations.ld


ENTRY(_start)


SECTIONS
{
    . = BOOT_LOADER_BASE;

    /*
     * Just keep a stub function at the start of the memory space.
     */

    .text : {
        KEEP(*(.text._start))
    }


    /*
     * Put the real bootloader in a safe place that's separate from where the kernel itself will be
     * loaded.
     */

    . = BOOT_ELF_LOADER_BASE;
    _elf_loader_start = .;

    .elf_loader : {
        *(.text*)
    }

    .rodata : {
        *(.rodata*)
        *(.srodata*)
    }

    .data : {
        *(.data*)
        *(.sdata*)
    }

    . = ALIGN(16);
    _bss_start = .;
    .bss  : {
        *(.bss*)
        *(.sbss*)
        *(COMMON)
    }
    . = ALIGN(16);
    _bss_end = .;


    /*
     * The stack needs to be safe from the ELF loader as well.
     */

    . = ALIGN(16);
    _stack_bottom = .;
    . += 0x400000;
    _stack_top = .;

    _elf_loader_end = .;


    /*
     * Make sure the ELF loader fits inside of it's assigned space.
     */

    ASSERT(_elf_loader_end <= BOOT_LOADER_END,
           "Relocated ELF loader overflowed heap-resident region")

    /DISCARD/ : {
        *(.eh_frame)
        *(.comment)
    }
}
