
# ----- XTRA-OS CMake Configuration ----------------------------------------------------------------
# This CMake configuration is for building the XTRA-OS.
#
# Generate the build system using:
#
#     cmake .. \
#           -DCMAKE_C_COMPILER=riscv64-linux-gnu-gcc \
#           -DCMAKE_CXX_COMPILER=riscv64-linux-gnu-g++ \
#           -G Ninja


cmake_minimum_required(VERSION 3.10)

project(xtra-os)


set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)


enable_language(ASM)


set(SYSTEM_SRC_DIR "${CMAKE_CURRENT_SOURCE_DIR}/src")
set(SYSTEM_TREE_DIR "${CMAKE_CURRENT_SOURCE_DIR}/system-tree")
set(SYSTEM_ROOT_FS_DIR "${SYSTEM_TREE_DIR}/root-fs")



# ----- Bootloader ---------------------------------------------------------------------------------

set(BOOTLOADER_SRC "${SYSTEM_SRC_DIR}/loader")

add_executable(bootloader.elf
               "${BOOTLOADER_SRC}/start.s"
               "${BOOTLOADER_SRC}/boot.cpp")

set_property(TARGET bootloader.elf PROPERTY
             LINK_DEPENDS "${BOOTLOADER_SRC}/linker.ld")

set_target_properties(bootloader.elf PROPERTIES
                      RUNTIME_OUTPUT_DIRECTORY ${SYSTEM_TREE_DIR})

target_compile_options(bootloader.elf PRIVATE
                       -ffreestanding
                       -nostdlib
                       -Wall
                       -Wextra
                       -march=rv64gc
                       -mabi=lp64
                       -fno-exceptions
                       -fno-rtti)

target_link_options(bootloader.elf PRIVATE
                    -nostdlib
                    -static
                    -T "${BOOTLOADER_SRC}/linker.ld"
                    -Wl,--build-id=none
                    -Wl,--no-warn-rwx-segments)



# ----- Kernel -------------------------------------------------------------------------------------

set(KERNEL_SRC "${SYSTEM_SRC_DIR}/kernel")

add_executable(kernel.elf
               "${KERNEL_SRC}/start.s"
               "${KERNEL_SRC}/kernel.cpp")

set_property(TARGET kernel.elf PROPERTY
             LINK_DEPENDS "${KERNEL_SRC}/linker.ld")

set_target_properties(kernel.elf PROPERTIES
                      RUNTIME_OUTPUT_DIRECTORY "${SYSTEM_TREE_DIR}/boot")

target_compile_options(kernel.elf PRIVATE
                       -ffreestanding
                       -nostdlib
                       -Wall
                       -Wextra
                       -march=rv64gc
                       -mabi=lp64
                       -fno-exceptions
                       -fno-rtti)

target_link_options(kernel.elf PRIVATE
                    -nostdlib
                    -static
                    -T "${KERNEL_SRC}/linker.ld"
                    -Wl,--build-id=none
                    -Wl,--no-warn-rwx-segments)



# ----- User-Land ----------------------------------------------------------------------------------

add_custom_command(OUTPUT ${SYSTEM_ROOT_FS_DIR}
                   COMMAND mkdir -p ${SYSTEM_ROOT_FS_DIR}/bin
                   COMMAND mkdir -p ${SYSTEM_ROOT_FS_DIR}/home
                   COMMAND mkdir -p ${SYSTEM_ROOT_FS_DIR}/dev
                   COMMAND mkdir -p ${SYSTEM_ROOT_FS_DIR}/lib
                   COMMAND mkdir -p ${SYSTEM_ROOT_FS_DIR}/tmp
                   COMMAND mkdir -p ${SYSTEM_ROOT_FS_DIR}/etc
                   COMMENT "Creating root filesystem directory structure...")



# ----- Disk Image Building ------------------------------------------------------------------------

# Create the raw partitioned disk image.
add_custom_command(OUTPUT ${SYSTEM_TREE_DIR}/disk0.img
                   COMMAND dd if=/dev/zero of=${SYSTEM_TREE_DIR}/disk0.img bs=1M count=1024
                   COMMAND parted -s ${SYSTEM_TREE_DIR}/disk0.img mklabel msdos
                   COMMAND parted -s ${SYSTEM_TREE_DIR}/disk0.img mkpart primary fat32 1MiB 513MiB
                   COMMAND parted -s ${SYSTEM_TREE_DIR}/disk0.img mkpart primary ext2 513MiB 100%
                   COMMAND parted -s ${SYSTEM_TREE_DIR}/disk0.img set 1 boot on
                   COMMENT "Creating 1GB disk0.img with partition table...")

# Create the FAT32 boot partition image.
add_custom_command(OUTPUT ${SYSTEM_TREE_DIR}/disk0-part0.img
                   COMMAND dd if=/dev/zero of=${SYSTEM_TREE_DIR}/disk0-part0.img bs=1M count=512
                   COMMAND mkfs.fat -F 32 ${SYSTEM_TREE_DIR}/disk0-part0.img
                   COMMAND mcopy -i ${SYSTEM_TREE_DIR}/disk0-part0.img ${SYSTEM_TREE_DIR}/boot/kernel.elf ::kernel.elf
                   DEPENDS kernel.elf
                   COMMENT "Creating 512MB FAT32 boot partition...")

# Create the ext2 root filesystem image.
add_custom_command(OUTPUT ${SYSTEM_TREE_DIR}/disk0-part1.img
                   COMMAND genext2fs -d ${SYSTEM_ROOT_FS_DIR} -b 524288 ${SYSTEM_TREE_DIR}/disk0-part1.img
                   DEPENDS ${SYSTEM_ROOT_FS_DIR}
                   COMMENT "Creating ext2 root filesystem...")

# Create the disk image combining both partitions.
add_custom_command(OUTPUT ${SYSTEM_TREE_DIR}/disk0.img.complete
                   COMMAND dd if=${SYSTEM_TREE_DIR}/disk0-part0.img of=${SYSTEM_TREE_DIR}/disk0.img bs=1M seek=1 conv=notrunc
                   COMMAND dd if=${SYSTEM_TREE_DIR}/disk0-part1.img of=${SYSTEM_TREE_DIR}/disk0.img bs=1M seek=513 conv=notrunc
                   COMMAND touch ${SYSTEM_TREE_DIR}/disk0.img.complete
                   DEPENDS ${SYSTEM_TREE_DIR}/disk0.img ${SYSTEM_TREE_DIR}/disk0-part0.img ${SYSTEM_TREE_DIR}/disk0-part1.img
                   COMMENT "Assembling final disk0.img...")

# Create the final disk image target.
add_custom_target(disk-image DEPENDS ${SYSTEM_TREE_DIR}/disk0.img.complete)



# ----- User Home Image Building -------------------------------------------------------------------



# ----- QEMU Execution -----------------------------------------------------------------------------

add_custom_target(run
                  COMMAND qemu-system-riscv64
                          -machine virt
                          -bios none
                          -kernel ${SYSTEM_TREE_DIR}/bootloader.elf
                          -drive file=${SYSTEM_TREE_DIR}/disk0.img,format=raw,id=hd0
                          -device virtio-blk-device,drive=hd0
                          -serial stdio
                          -display sdl
                          -smp 1
                          -m 1024M
                  DEPENDS bootloader.elf disk-image
                  WORKING_DIRECTORY ${SYSTEM_TREE_DIR}
                  USES_TERMINAL
                  COMMENT "Running XTRA-OS in QEMU...")
