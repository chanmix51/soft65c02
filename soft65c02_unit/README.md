# soft65c02_unit

A test runner and build system for 6502/65C02 assembly and C code that integrates with the soft65c02 emulator. This tool compiles your code using configurable toolchains (like CC65) and runs automated tests using a domain-specific language (DSL) for hardware emulation testing.

## Building and Installing

To build, create executable, and install into local `~/.cargo/bin` folder:

```shell
# from the project root dir
cargo build --workspace --release
cargo install --path soft65c02_tester
cargo install --path soft65c02_unit
```

## Quick Start

A complete test setup consists of three files:

1. **Source code** (`.s` or `.c`) - Your 6502/65C02 program
2. **Build configuration** (`.yaml`) - Defines how to compile your code  
3. **Test script** (`.txt`) - Automated tests using the soft65c02 DSL

When compiling the test for running in the emulator, you need a folder for the build artifacts
to be written to. This can be anywhere, including some tmp folder. Examples below just use a simple
local folder.

Example minimal setup:
```bash
cd soft65c02_unit/examples/simple_test/tests

# Compile and run tests
soft65c02_unit -i my_test.yaml -b /tmp/my-build

# Or with verbose output
soft65c02_unit -v -i my_test.yaml -b /tmp/my-build

# Using environment variable for build directory
export SOFT65C02_BUILD_DIR=/tmp/build
soft65c02_unit -i my_test.yaml
```

See `examples/simple_test/` for a complete working example.

## Command Line Usage

```bash
soft65c02_unit [OPTIONS] --input <INPUT>
```

### Required Arguments

**`-i, --input <INPUT>`**  
Path to the test YAML configuration file that defines your build and test setup.

```bash
soft65c02_unit -i tests/my_test.yaml
```

**Build Directory**  
You must specify where build artifacts should be stored, either via command line or environment variable:

```bash
# Option 1: Command line argument
soft65c02_unit -i my_test.yaml -b /tmp/build

# Option 2: Environment variable
export SOFT65C02_BUILD_DIR=/tmp/build
soft65c02_unit -i my_test.yaml
```

### Options

**`-b, --build-dir <BUILD_DIR>`**  
Directory for build outputs (object files, binaries, symbol files). Creates the directory if it doesn't exist. Required unless `SOFT65C02_BUILD_DIR` environment variable is set.

**`-v, --verbose`**  
Enable verbose output showing:
- Detailed compilation commands
- Step-by-step execution traces
- Memory and register states during testing
- Build process information

```bash
soft65c02_unit -v -i my_test.yaml -b ./build
```

**`--dry-run`**  
Print all commands that would be executed without actually running them. Useful for:
- Debugging build configurations
- Verifying compiler command generation
- Testing new configurations safely

```bash
soft65c02_unit --dry-run -i my_test.yaml -b ./build
```

**`-h, --help`**  
Display help information and usage examples.

**`-V, --version`**  
Print the current version of soft65c02_unit.

### Environment Variables

**`SOFT65C02_BUILD_DIR`**  
Default build directory when `-b/--build-dir` is not specified. Useful for consistent builds across multiple test runs:

```bash
# Set once, use everywhere
export SOFT65C02_BUILD_DIR=$PWD/build
soft65c02_unit -i test1.yaml
soft65c02_unit -i test2.yaml
soft65c02_unit -i test3.yaml
```

### Example Usage Patterns

```bash
# Basic test run
soft65c02_unit -i tests/basic.yaml -b ./build

# Verbose debugging of failing test
soft65c02_unit -v -i tests/debug.yaml -b ./debug_build

# Check build commands without executing
soft65c02_unit --dry-run -i tests/complex.yaml -b ./build

# Development workflow with persistent build dir
export SOFT65C02_BUILD_DIR=./build
soft65c02_unit -i tests/unit_test.yaml        # Quick iteration
soft65c02_unit -v -i tests/integration.yaml   # Detailed output when needed
```

## File Structure

### Source Files (`.s` / `.c`)

Your 6502/65C02 source code written in assembly or C. The build system supports:
- **Assembly files** (`.s`) - 6502/65C02 assembly using CC65 syntax
- **C files** (`.c`) - C code that compiles to 6502/65C02 using CC65

Example assembly (`src/main.s`):
```assembly
.include "atari.inc"

.export start
.proc start
    lda #$42
    sta COLOR2
    rts
.endproc
```

Example C (`src/main.c`):
```c
#include <atari.h>

int main() {
    POKE(COLOR2, 0x42);
    return 0;
}
```

### Build Configuration (`.yaml`)

Defines how to compile your code and where to find dependencies. The YAML file specifies:

- **Compiler settings** - Target platform, toolchain, configuration files
- **Source files** - Which files to compile and in what order
- **Include paths** - Where to find headers and assembly includes  
- **Compiler flags** - Custom flags for C, assembly, and linking
- **Test script** - Which DSL test file to run

Example configuration (`my_test.yaml`):
```yaml
name: "my_test"
target: "atari"
compiler: cc65
config_file: "base_configs/config_min_atari.cfg"

src_files:
  - "src/main.s"
  - "src/utils.c"

include_paths:
  - "include"
  
asm_include_paths:
  - "include"

test_script: "tests/my_test.txt"

# Optional compiler flags
c_flags:
  - "-O"        # Enable optimizations
  - "-Wall"     # Enable warnings
  
asm_flags:
  - "-g"        # Include debug info
  
ld_flags:
  - "-m"        # Generate map file
  - "game.map"
```

### Test Script (`.txt`)

Automated tests written in the soft65c02 DSL. These scripts can:
- Set up memory and registers
- Load your compiled binary
- Execute code step-by-step or until conditions are met
- Assert expected behavior
- Test edge cases and error conditions

Example test script (`tests/my_test.txt`):
```
marker $$basic functionality test$$

memory load ${BINARY_PATH}
symbols load ${SYMBOLS_PATH}

registers set CP=$start
run until false

assert #0x302 = 0x42  $$color register set correctly$$
```

## YAML Configuration

### File Inclusion and Composition

YAML configurations support modular composition through the `configs` field. This allows you to:
- Share common settings across multiple tests
- Separate platform-specific configuration from project logic
- Build hierarchical configuration systems

```yaml
# Platform configuration (atari_base.yaml)
compiler: cc65
target: atari
config_file: "platform/atari.cfg"
include_paths:
  - "platform/include"

# Project configuration (my_project.yaml) 
configs:
  - "atari_base.yaml"  # Include platform settings

name: "my_project"
src_files:
  - "src/main.s"
include_paths:
  - "src/include"      # Added to platform includes
```

**Configuration Loading Order:**
1. Dependencies are loaded first (depth-first)
2. Current file settings override inherited ones
3. Lists (like `src_files`, `include_paths`) are **combined** from all configs
4. Simple values (like `name`, `target`) use the **last defined** value

**List Combination Example:**
```yaml
# base.yaml
include_paths:
  - "platform/include"
src_files:
  - "lib/common.s"

# project.yaml  
configs:
  - "base.yaml"
include_paths:
  - "src/include"     # Combined with base
src_files:
  - "src/main.s"      # Combined with base

# Result: include_paths = ["platform/include", "src/include"]
#         src_files = ["lib/common.s", "src/main.s"]
```

### Environment Variables

Environment variables can be used in YAML files using `${VARIABLE_NAME}` syntax:

```yaml
name: "${PROJECT_NAME}"
include_paths:
  - "${SDK_PATH}/include"
config_file: "${PLATFORM_CONFIG}/atari.cfg"
```

**Automatically Defined Variables:**

The test runner automatically defines these environment variables for use in test scripts:

- **`BINARY_PATH`** - Path to the compiled binary file
- **`SYMBOLS_PATH`** - Path to the generated symbols file (for debugging)

These are particularly useful in test scripts:
```
# In your test script (.txt file)
memory load ${BINARY_PATH}    $$load the compiled program$$
symbols load ${SYMBOLS_PATH}  $$load symbol table for debugging$$
```

## Compiler Support

### CC65 Toolchain

The primary supported compiler is **CC65**, a complete cross-development package for 6502/65C02 systems. CC65 provides:

- **C compiler** - Full ANSI C support optimized for 6502
- **Assembler** - Native 6502/65C02 assembly with macro support  
- **Linker** - Flexible memory layout configuration
- **Target platforms** - Built-in support for Apple II, Atari, C64, NES, and more

Configuration example:
```yaml
compiler: cc65
target: "atari"                    # Target platform
config_file: "platform/atari.cfg" # Memory layout configuration

# Compiler-specific flags
c_flags:
  - "-O"          # Enable optimizations
  - "-g"          # Include debug information
  - "-Wall"       # Enable all warnings
  - "-DDEBUG=1"   # Define preprocessor macros

asm_flags:
  - "-g"              # Include debug information  
  - "--auto-import"   # Auto-import symbols
  - "-DDEBUG"         # Define assembly symbols

ld_flags:
  - "-m"          # Generate map file
  - "game.map"    # Map file name
  - "--dbgfile"   # Generate debug file
  - "game.dbg"    # Debug file name
```

### Extensible Architecture

The compiler system is designed to be extensible. While CC65 is currently the only implemented compiler, the architecture supports adding other toolchains:

- **Modular design** - Each compiler implements a common `Compiler` trait
- **Configurable execution** - Compilers can use different executables and flag formats
- **Platform abstraction** - Target platforms are abstracted from specific toolchain details

Future compiler support could include:
- **ACME** assembler
- **DASM** assembler  
- **Custom toolchains** for specific hardware

To implement a new compiler, create a struct implementing the `Compiler` trait with methods for:
- `compile_source()` - Compile individual source files
- `link_objects()` - Link object files into final binary
- `get_symbols_path()` - Provide symbol file location

## Test DSL Reference

Test scripts use the soft65c02 domain-specific language (DSL) for hardware emulation testing. The DSL provides comprehensive capabilities for:

- **Memory manipulation** - Load binaries, write data, fill ranges
- **Register control** - Set CPU registers and flags  
- **Execution control** - Run code step-by-step or until conditions
- **Assertions** - Verify expected behavior and state
- **Symbol support** - Use labels and named addresses
- **Debugging** - Disassemble code and inspect memory

**Complete DSL documentation is available in:** [`soft65c02_tester/documentation.md`](../soft65c02_tester/documentation.md)

Key DSL features:
```
marker $$test description$$               # Start new test
memory load ${BINARY_PATH}                # Load compiled binary
symbols load ${SYMBOLS_PATH}              # Load symbol table
registers set A=0x42                      # Set register values
run #0x2000                              # Execute from address
run until A = 0x00                       # Run until condition
assert #0x2000 = 0xa9                    # Assert memory value
assert A > 0x7f                          # Assert register condition
disassemble $main 0x20                   # Show disassembly
```

## Environment Variables

### Automatic Variables

These variables are automatically set by the test runner and available in both YAML configurations and test scripts:

| Variable | Description | Example Value |
|----------|-------------|---------------|
| `BINARY_PATH` | Path to compiled binary | `./build/game.bin` |
| `SYMBOLS_PATH` | Path to symbol/label file | `./build/game.lbl` |

### Configuration Variables

These environment variables control the behavior of soft65c02_unit:

| Variable | Description | Usage |
|----------|-------------|-------|
| `SOFT65C02_BUILD_DIR` | Default build directory | `export SOFT65C02_BUILD_DIR=./build` |

### Using Environment Variables

**In YAML configurations:**
```yaml
# Reference external tools or paths
sdk_path: "${CC65_HOME}/lib"
config_file: "${PLATFORM_CONFIGS}/atari.cfg"

# Use in include paths
include_paths:
  - "${CC65_HOME}/include"
  - "${PROJECT_ROOT}/include"
```

**In test scripts:**
```
memory load ${BINARY_PATH}
symbols load ${SYMBOLS_PATH}

# Can also reference custom environment variables
memory load ${TEST_DATA_PATH}/sample.bin
```

## Examples

### Complete Example

See `examples/simple_test/` for a complete working example that demonstrates:

- **Multi-file project** with C and assembly sources
- **Hierarchical configuration** using base configs
- **Comprehensive testing** with setup, execution, and assertions
- **Symbol usage** for readable test scripts
- **Build artifacts** and debugging information

```bash
cd examples/simple_test
soft65c02_unit -v -i tests/my_test.yaml -b ./build
```

### Directory Structure
```
examples/simple_test/
├── base_configs/
│   ├── atari.yaml              # Platform configuration
│   └── config_min_atari.cfg    # CC65 memory layout
├── src/
│   ├── fn_under_test.s         # Assembly function
│   ├── test_runner.s           # Test harness
│   └── bar/util.s              # Utility functions
├── tests/
│   ├── my_test.txt             # DSL test script
│   └── my_test.yaml            # Build configuration
```

### Build Configuration with Flags
```yaml
configs:
  - "base_configs/atari.yaml"

name: "comprehensive_test"
src_files:
  - "src/test_runner.s"
  - "src/fn_under_test.s"
  - "src/bar/util.s"

# Optimized build with debugging
c_flags:
  - "-O2"
  - "-g"
  - "-Wall"
  - "-DRELEASE_BUILD"

asm_flags:
  - "-g"
  - "--auto-import"

ld_flags:
  - "-m"
  - "build.map"

test_script: "tests/comprehensive_test.txt"
```

## Installation

To install into `$HOME/.cargo/bin`:

```bash
cargo install --path soft65c02_unit
```

### Dependencies

- **Rust toolchain** (1.70+)
- **CC65** cross-development package
  - Ubuntu/Debian: `apt install cc65`
  - macOS: `brew install cc65`
  - Windows: Download from [cc65.github.io](https://cc65.github.io/)

### Development Build

```bash
# Build from source
cargo build --release

# Run tests
cargo test

# Run with local build
./target/release/soft65c02_unit -i examples/simple_test/tests/my_test.yaml -b ./build
```
