
    .section .text.boot, "ax"
    .globl _start


_start:
    # Write a character to UART to prove _start is running
    li   t3, 0x10000005      # UART0 LSR
uart_wait:
    lb   t4, 0(t3)
    andi t4, t4, 0x20
    beqz t4, uart_wait

    li   t3, 0x10000000      # UART0 THR
    li   t4, 65              # ASCII 'A'
    sb   t4, 0(t3)

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
    .skip  4096 # Reserve 4KB for the stack.
_stack_top:
