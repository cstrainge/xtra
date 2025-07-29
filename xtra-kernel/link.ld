
ENTRY(_start)


__stack_size = 0x1000;     /* 4 KiB stack size per hart. */
__num_harts = 4;           /* We support a maximum of 4 harts. */


SECTIONS
{
    /*
        The bootloader starts at this 0x80000000, and the kernel is loaded at 0x80500000 5MB after
        that.

        Once loaded and initialized, the kernel will repurpose that initial 5MB as it's heap.
    */

    . = 0x80000000;

    _heap_start = .;

    . = 0x80500000;

    _heap_end = .;

    _kernel_start = .;

    .text : ALIGN(0x1000)
    {
        _text_start = .;

        KEEP(*(.text._start))
        *(.text*)

        _text_end = .;
    }

    . = ALIGN(0x1000);

    .rodata : ALIGN(0x1000)
    {
        _rodata_start = .;

        *(.rodata*)

        _rodata_end = .;
    }

    . = ALIGN(0x1000);

    .data : ALIGN(0x1000)
    {
        _data_start = .;

        *(.data*)

        _data_end = .;
    }

    . = ALIGN(0x1000);

    .bss : ALIGN(0x1000)
    {
        _bss_start = .;

        *(.bss*)

        _bss_end = .;
    }

    . = ALIGN(0x1000);

    .stacks (NOLOAD) : ALIGN(0x1000)
    {
        PROVIDE(_stack_start = .);
        . = . + (__stack_size * __num_harts);
        PROVIDE(_stack_end = .);
    }

    _kernel_end = .;

    /DISCARD/ :
    {
        *(.eh_frame .comment)
    }
}
