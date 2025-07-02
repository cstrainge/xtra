
ENTRY(_start)

SECTIONS
{
    . = 0x80000000;

    .text : {
        KEEP(*(.text._start))
        *(.text*)
    }

    .rodata : { *(.rodata*) }
    .data : { *(.data*) }
    .bss  : { *(.bss*) }
    .bss : { *(.bss*) }
    /DISCARD/ : { *(.eh_frame .comment) }

    _stack_start = . + 0x400000; /* top-of-stack symbol */
}
