#!/usr/bin/env bash


# ---- Build the bootloader ------------------------------------------------------------------------
cargo build --target riscv64imac-unknown-none-elf --release
if [ $? -ne 0 ]
then
    echo "Build failed."
    exit 1
fi

mkdir -p build
cp target/riscv64imac-unknown-none-elf/release/xtra-bootloader build/xtra-bootloader
if [ $? -ne 0 ]
then
    echo "Copy failed."
    exit 1
fi


# ---- Run the OS in QEMU --------------------------------------------------------------------------
qemu-system-riscv64 \
    -machine virt \
    -bios none \
    -kernel build/xtra-bootloader \
    -serial stdio \
    -display sdl \
    -smp 2 \
    -m 1024M

#    -drive file=${SYSTEM_TREE_DIR}/disk0.img,format=raw,id=hd0 \
#    -device virtio-blk-device,drive=hd0 \
