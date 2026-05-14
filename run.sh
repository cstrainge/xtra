#!/usr/bin/env bash

set -euo pipefail



# ---- Configure the build. ------------------------------------------------------------------------

# Default to release build if no argument is provided.
BUILD_MODE=${1:-release}

# Let the user know which build mode is being used.
echo "Building the OS in $BUILD_MODE mode..."



# ---- Build the bootloader and the Kernel. --------------------------------------------------------

# Compile the bootloader and the Kernel fo the RISC-V 64-bit architecture. Right now the code
# supports QEMU devices drivers. The bootloader in particular is built with QEMU in mind.
if [ "$BUILD_MODE" == "debug" ]
then
    cargo build --target riscv64imac-unknown-none-elf --workspace --features nightly
else
    cargo build --target riscv64imac-unknown-none-elf --workspace --features nightly --release
fi

# Copy the bootloader into place for QEMU to load.
mkdir -p build
cp target/riscv64imac-unknown-none-elf/$BUILD_MODE/xtra-bootloader build/xtra-bootloader

# Copy the Kernel into place for the bootloader to load from the disk image.
mkdir -p build/boot
cp target/riscv64imac-unknown-none-elf/$BUILD_MODE/xtra-kernel build/boot/kernel.elf

# Create the mount table for the Kernel to know how to mount the base disk partitions. The
# bootloader will parse this file and pass the information to the Kernel so that it can mount the
# required filesystems.
#
# Later if we add loadable device drivers the Kernel can search for them in the /boot/ directory.
cat > build/boot/mount.tbl <<'EOF'
mount /     disk:0 pt:1 type:ext2
mount /boot disk:0 pt:0 type:fat32
EOF



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



# ---- Create the system disk images. --------------------------------------------------------------

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
mcopy -i build/disk0-part0.img build/boot/mount.tbl ::mount.tbl

# Create the ext2 root filesystem image (~990MB)
genext2fs -d build/sys-root -b 253952 build/disk0-part1.img

# Combine partitions into disk image
dd if=build/disk0-part0.img of=build/disk0.img bs=1M seek=1 conv=notrunc
dd if=build/disk0-part1.img of=build/disk0.img bs=1M seek=33 conv=notrunc



# ---- Run the OS in QEMU. -------------------------------------------------------------------------

START_BIN="build/xtra-bootloader"

qemu-system-riscv64 \
    -machine virt \
    -bios none \
    -kernel $START_BIN \
    -global virtio-mmio.force-legacy=false \
    -drive file=build/disk0.img,if=none,format=raw,id=x0 \
    -device virtio-blk-device,drive=x0,bus=virtio-mmio-bus.0 \
    -serial stdio \
    -display sdl \
    -smp 4 \
    -m 2048M
