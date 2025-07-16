
ENTRY(_start)


__stack_size = 0x1000;     /* 4 KiB stack size per hart. */
__num_harts = 4;           /* We support a maximum of 4 harts. */


SECTIONS
{
    . = 0x80500000;

    _kernel_start = .;

    .text :
    {
        _text_start = .;

        KEEP(*(.text._start))
        *(.text*)

        _text_end = .;
    }

    .rodata :
    {
        _rodata_start = .;

        *(.rodata*)

        _rodata_end = .;
    }

    .data :
    {
        _data_start = .;

        *(.data*)

        _data_end = .;
    }

    .bss  :
    {
        _bss_start = .;

        *(.bss*)

        _bss_end = .;
    }

    .stacks (NOLOAD) : ALIGN(16)
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
