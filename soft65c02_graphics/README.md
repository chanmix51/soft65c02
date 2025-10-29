# Soft65C02 Graphics Module

Graphics display backends for the Soft65C02 processor emulator.

## Overview

The `soft65c02_graphics` crate provides display backends for the Soft65C02 processor emulator. It implements memory-mapped graphics that emulate how real 8-bit computers handled video output - by treating display memory as addressable I/O.

This crate was separated from the core `soft65c02_lib` to maintain clean architecture and allow multiple graphics backends away from the core emulation library.

## Available Backends

### 1. MiniFB Backend (`minifb-backend` feature)
- **Library**: MiniFB for simple framebuffer rendering
- **Pros**: 
  - Pure software rendering (no GPU requirements)
  - Very lightweight
  - Simple API
- **Cons**: 
  - Known issues with Wayland on Linux
  - Less modern graphics pipeline

### 2. Pixels Backend (`pixels-backend` feature)
- **Library**: Pixels + Winit for modern cross-platform graphics
- **Pros**: 
  - Wayland support
  - Modern graphics pipeline using wgpu
  - Better cross-platform compatibility
  - Hardware acceleration where available
- **Cons**: 
  - Slightly higher system requirements
  - More dependencies

## Memory Layout

The graphics system uses a memory-mapped approach with the following layout:

```
0x0000 → 0x002F    Palette (16 colors × 3 RGB bytes = 48 bytes)
0x0030 → 0x003F    Keyboard input buffer (16 bytes) 
0x0040 → 0x00FF    Unused RAM space
0x0100 → 0x1900    Video buffer (128×96 pixels, 4-bit indexed, 6144 bytes)
```

### Display Specifications
- **Resolution**: 128×96 pixels (4:3 aspect ratio)
- **Color Depth**: 4-bit indexed (16 colors)
- **Palette**: 16 customizable RGB colors
- **Storage**: 2 pixels per byte (packed nibbles)
- **Scaling**: 4× upscaling for comfortable viewing (512×384 window)

## Usage

### Basic Usage

```rust
use soft65c02_graphics::MiniFBDisplay;  // or PixelsDisplay
use soft65c02_lib::{AddressableIO, Memory};

let mut memory = Memory::new_with_ram();
memory.add_subsystem("VIDEO TERMINAL", 0x0200, MiniFBDisplay::new());

// Set up a simple palette
let palette = vec![
    0x00, 0x00, 0x00,  // Black
    0xFF, 0xFF, 0xFF,  // White  
    0xFF, 0x00, 0x00,  // Red
    // ... more colors
];
memory.write(0x0200, &palette).unwrap();

// Draw pixels (2 pixels per byte, left=low nibble, right=high nibble)
memory.write(0x0300, &[0x12]).unwrap(); // Left pixel=color 2, right pixel=color 1
```

### Choosing a Backend

#### For Maximum Compatibility (Especially Wayland)
```toml
[dependencies]
soft65c02_graphics = { version = "1.0.0-alpha2", features = ["pixels-backend"] }
```

```rust
use soft65c02_graphics::PixelsDisplay;
let display = PixelsDisplay::new();
```

#### For Minimal Dependencies
```toml
[dependencies]
soft65c02_graphics = { version = "1.0.0-alpha2", features = ["minifb-backend"] }
```

```rust
use soft65c02_graphics::MiniFBDisplay;
let display = MiniFBDisplay::new();
```

## Examples

### Running Examples

**MiniFB Backend:**
```bash
cargo run --example minifb --features minifb-backend
```

**Pixels Backend (recommended for Wayland):**
```bash
cargo run --example pixels --features pixels-backend
```

Both examples will:
1. Create a 65C02 processor instance
2. Load a test program that fills the screen with patterns
3. Display the graphics window with palette-based rendering
4. Show processor registers in the terminal

### Example Program Output

The examples run a hand-assembled 65C02 program that:
- Sets up a brown color pattern across the screen
- Demonstrates memory-mapped video output
- Shows how the 65C02 writes to video memory addresses

## Architecture

The crate provides:
- `DisplayBackend` trait for graphics backend abstraction
- `AddressableIO` implementation for memory-mapped graphics
- Keyboard input handling with consistent key codes
- Separate thread management for graphics rendering
- Clean separation from the core CPU emulation

## Keyboard Input

Both backends provide the same keyboard input interface:
- Keys are mapped to consistent internal codes (0x01-0x6B)
- Input events are available through the `DisplayBackend::get_input_events()` method
- Supports alphanumeric keys, function keys, arrows, and modifiers

## Performance Notes

- **MiniFB**: Pure software rendering, consistent performance across systems
- **Pixels**: Uses hardware acceleration when available, may fall back to software rendering
- Both backends handle the 128×96 @ 4-bit graphics efficiently
- Memory-mapped updates only re-render changed regions

## Cross-Platform Support

- **Linux**: Both backends supported (Pixels recommended for Wayland)
- **Windows**: Both backends fully supported  
- **macOS**: Both backends fully supported
- **BSD/Unix**: MiniFB recommended

Choose the backend that best fits your target environment and requirements!

## Development

This crate is part of the Soft65C02 project. Both backends provide identical memory-mapped graphics interfaces, making it easy to switch between them based on your needs.

For issues specific to graphics rendering, please check:
- Wayland compatibility → Use Pixels backend
- Minimal dependencies → Use MiniFB backend  
- Performance issues → Both backends are optimized for the 65C02's graphics capabilities

## Dependencies

- `soft65c02_lib`: Core emulator library (required)
- `minifb`: Window management and rendering (MiniFB backend only)

## Technical Details

### Thread Safety
Graphics backends run in their own threads to maintain smooth rendering without blocking the CPU emulation. Communication happens through atomic operations and mutex-protected buffers.

### Performance
The memory-mapped approach allows for efficient bulk updates while maintaining the authentic feel of 8-bit computer graphics programming.

### Compatibility
This design maintains full compatibility with existing `soft65c02_lib` code while providing a clean separation of concerns.

## License

Same as the parent project (GPL v3).

## Contributing

When adding new graphics backends:

1. Implement both `AddressableIO` and `DisplayBackend` traits
2. Add appropriate feature flags in `Cargo.toml`
3. Include examples demonstrating the backend
4. Update this README with backend-specific documentation
5. Ensure thread safety and proper resource cleanup 