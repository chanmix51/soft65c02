#!/bin/bash -e

#
# 6502 Assembly Build Script for Graphics Demo
#
# This script compiles the 6502 assembly code that handles:
# - Real-time keyboard input polling and processing
# - Game mode selection and switching logic  
# - Memory-mapped communication with Rust graphics processors
# - Pause/resume functionality
#
# Build Process:
# 1. Compiles minimal C runtime (min_crt0.s) for system initialization
# 2. Compiles main game controller (game.s) with input handling logic
# 3. Links both objects using custom memory layout (game.cfg)
# 4. Produces game.bin binary loaded at 0x1000 by Rust host
#
# Output: build/game.bin (6502 machine code for memory-mapped game controller)
#

echo "Compiling Game Framebuffer for 65C02..."

if [ ! -d build ]; then
  mkdir build
fi

rm -f build/* >/dev/null

cl65 -t none -c --create-dep build/min_crt0.d --listing build/min_crt0.lst -o build/min_crt0.o min_crt0.s
cl65 -t none -c --create-dep build/game.d --listing build/game.lst -o build/game.o game.s
cl65 -t none -C game.cfg --mapfile build/game.map -Ln build/game.lbl -o build/game.bin build/game.o build/min_crt0.o

if [ $? -eq 0 ]; then
    echo "Compilation successful! Binary: game.bin"
    ls -la build/game.bin
else
    echo "Compilation failed!"
    exit 1
fi

echo "Done. You can now run the graphics demo from the project root:"
echo "cargo run --release --example game --features pixels-backend" 