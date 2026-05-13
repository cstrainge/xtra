
/* Core locations in the Kernel address space. */

RAM_BASE             = 0x80000000;
KERNEL_BASE          = RAM_BASE;


/* Where in memory the Kernel's heap is located. */

KERNEL_HEAP_BASE     = 0x80500000;
KERNEL_HEAP_SIZE     = 0X00500000;
KERNEL_HEAP_END      = KERNEL_HEAP_BASE + KERNEL_HEAP_SIZE;


/*
 * Locate the boot-loader's elf-loading code in the Kernel's heap so that when it loads the Kernel
 * itself it will be able to do so safely.
 *
 * The bootloader initially occupies the same space the Kernel will occupy, so the code that
 * actually overwrites the bootloader to load the kernel needs to be somewhere out of the way.
*/

BOOT_LOADER_BASE     = RAM_BASE;
BOOT_ELF_LOADER_BASE = KERNEL_HEAP_BASE;
BOOT_LOADER_END      = KERNEL_HEAP_END;
