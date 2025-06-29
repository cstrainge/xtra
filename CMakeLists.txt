
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



# ----- Bootloader ---------------------------------------------------------------------------------

set(BOOTLOADER_SRC "${SYSTEM_SRC_DIR}/loader")

add_executable(bootloader.elf
               "${BOOTLOADER_SRC}/start.s"
               "${BOOTLOADER_SRC}/boot.cpp")

# Ensure bootloader gets rebuilt when linker script changes
set_property(TARGET bootloader.elf PROPERTY LINK_DEPENDS "${BOOTLOADER_SRC}/linker.ld")

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

# ----- User-Land ----------------------------------------------------------------------------------

# ----- Disk Image Building ------------------------------------------------------------------------

# ----- User Home Image Building -------------------------------------------------------------------

# ----- QEMU Execution -----------------------------------------------------------------------------

add_custom_target(run
                  COMMAND qemu-system-riscv64
                          -machine virt
                          -kernel ${SYSTEM_TREE_DIR}/bootloader.elf
                          -bios none
                          -serial stdio
                          -display sdl
                          -smp 1
                          -m 1024M
                  DEPENDS bootloader.elf
                  WORKING_DIRECTORY ${SYSTEM_TREE_DIR}
                  USES_TERMINAL
                  COMMENT "Running XTRA-OS in QEMU...")
