
ENTRY(_start)


SECTIONS
{
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
