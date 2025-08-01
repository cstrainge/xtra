#!/usr/bin/env bash

set -euo pipefail



# ---- Build the bootloader and the Kernel ---------------------------------------------------------

cargo build --target riscv64imac-unknown-none-elf --release

mkdir -p build
cp target/riscv64imac-unknown-none-elf/release/xtra-bootloader build/xtra-bootloader

mkdir -p build/boot
cp target/riscv64imac-unknown-none-elf/release/xtra-kernel build/boot/kernel.elf



# ---- Setup the user space directories. -----------------------------------------------------------

mkdir -p build/sys-root/bin
mkdir -p build/sys-root/boot
mkdir -p build/sys-root/home
mkdir -p build/sys-root/dev
mkdir -p build/sys-root/lib
mkdir -p build/sys-root/tmp
mkdir -p build/sys-root/etc
mkdir -p build/sys-root/proc
mkdir -p build/sys-root/mnt



# ---- Create the system disk images ---------------------------------------------------------------

# Create the partitioned disk image.
dd if=/dev/zero of=build/disk0.img bs=1M count=1024
parted -s build/disk0.img mklabel msdos
parted -s build/disk0.img mkpart primary fat32 1MiB 33MiB
parted -s build/disk0.img mkpart primary ext2 33MiB 100%
parted -s build/disk0.img set 1 boot on

# Create the FAT32 boot partition image (32MB)
dd if=/dev/zero of=build/disk0-part0.img bs=1M count=32
mkfs.fat -F 32 build/disk0-part0.img
mcopy -i build/disk0-part0.img build/boot/kernel.elf ::kernel.elf

# Create the ext2 root filesystem image (~990MB)
genext2fs -d build/sys-root -b 253952 build/disk0-part1.img

# Combine partitions into disk image
dd if=build/disk0-part0.img of=build/disk0.img bs=1M seek=1 conv=notrunc
dd if=build/disk0-part1.img of=build/disk0.img bs=1M seek=33 conv=notrunc



# ---- Run the OS in QEMU --------------------------------------------------------------------------

# START_BIN="build/xtra-bootloader"
START_BIN="target/riscv64imac-unknown-none-elf/debug/xtra-kernel"

qemu-system-riscv64 \
    -machine virt \
    -bios none \
    -kernel $START_BIN \
    -global virtio-mmio.force-legacy=false \
    -drive file=build/disk0.img,if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -serial stdio \
    -display sdl \
    -smp 1 \
    -m 2048M
