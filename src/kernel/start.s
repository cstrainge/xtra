
    .section .text
    .global _start


_start:
    # Setup the stack pointer.
    la   sp, _stack_top

    # Zero out the BSS section.
    la   t0, _bss_start
    la   t1, _bss_end
    li   t2, 0

1:
    bge  t0, t1, 2f
    sw   t2, 0(t0)      # Store zero in the BSS section
    addi t0, t0, 4      # Move to the next word
    j    1b             # Repeat until the end of BSS

2:
    # Call the main function.
    #   a0 = hart_id
    #   a1 = dtb_address
    call main

3:
    # if main returns we will just loop forever.
    wfi
    j 3b


    .section .bss
    .align 12

_stack_bottom:
    .skip 16384
_stack_top:
