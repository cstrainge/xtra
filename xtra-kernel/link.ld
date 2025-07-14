
ENTRY(_start)


SECTIONS
{
    . = 0x80500000;

    .text : {
        KEEP(*(.text._start))
        *(.text*)
    }

    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss  : { *(.bss*) }
    .bss : { *(.bss*) }
    /DISCARD/ : { *(.eh_frame .comment) }
}
