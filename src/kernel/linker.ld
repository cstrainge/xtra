
ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    .text : {
        *(.text)
    }

    .rodata : {
        *(.rodata*)
    }

    .data : {
        *(.data)
    }

    .bss : {
        _bss_start = .;
        *(.bss*)
        *(COMMON)
        _bss_end = .;
    }

    /DISCARD/ : { *(.eh_frame) }
}
