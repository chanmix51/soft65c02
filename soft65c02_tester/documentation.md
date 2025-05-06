# Soft65C02 tester

## Tester language

The parser language description is available [here](rules.pest).

### Comments

Comments can be used to add explanations or temporarily disable instructions. Two comment styles are supported:

```
// Traditional C-style comments
; Assembly-style comments
```

Comments can be used on their own lines:

```
// This is a comment
; This is also a comment
memory flush
; Clear memory before test
memory write #0x1234 0x(00,01,02)
```

Both comment styles consume the entire line from the comment marker to the end of the line. Empty lines and comment-only lines are ignored during execution.

### marker

```
marker $$marker description$$
```

The `marker` keyword initialize a new test plan. It sets the registers in a random state as the 65C02 when it starts and initializes the memory with 0x00 values.

There can be several test plans in a test script. Unless `continue_on_failure` parameter is set, if an assertion fails in a test plan the rest of the instructions will be ignored until the next `marker` keyword (or the end of the script) is reached.

### memory

The `memory` instructions are meant to write bytes to memory in order to prepare test environment.

#### memory flush

```
memory flush
```

Reset the whole memory with `0x00` values.

#### memory load

```
memory load #0x1234 "filename"
```

Load the given file into memory at the given address. If the file overflows the memory, an error is raised and the program stops.
This is the preferred way to load programs to be tested into the tester environment.

#### memory load atari

```
memory load atari "filename.xex"
```

Loads the given file into memory, as an Atari binary, honouring segments indicating their loading
locations.

#### memory load apple (Apple Single Format)

```
memory load apple "filename.com"
```

Loads the given file into memory, as an Apple Single ProDos file.
The loading address is read from the file.


#### memory write

```
memory write #0x1234 0x(00,01,02,…)
```

Write a slice of contiguous bytes at the given address.

Additionally writing supports strings, including escaped chars `\t`, `\n`, `\r`, `\0`, hexadecimal escape sequences `\xAA` where `AA` is a two-digit hex value (e.g., `\x0D`, `\xFF`), and line continuation with `\` at the end of a line.

Combined with symbols, you can write memory to named addresses with following:

```
memory write $main "hello, world\0"
memory write #0x2000 "data:\x0A\xFF\x00"   $$mix text with hex values$$

// Multi-line strings with line continuation
memory write $screen_memory "\
+-------+\
| Hello |\
+-------+"

// Equivalent to: memory write $screen_memory "+-------+| Hello |+-------+"
```

You can also write a memory location's address as two bytes (in little-endian format):

```
memory write $destination $src_address
```

This writes the 16-bit address value to memory. For example, if `$src_address` is `0x1234`, it will write `0x34` followed by `0x12` to 2 bytes located at `$destination`. This is particularly useful for setting up pointers and jump tables. The address can include offsets:

```
memory write $jump_table $handler+0x20   $$write address of handler+0x20 to jump_table$$
```

#### memory show

The `memory show` command displays a formatted hex dump of memory. It takes a location, length, optional width, and optional description:

```
memory show #0x1000 0x10           $$show 16 bytes starting at 0x1000$$
memory show $data 0x100            $$show 256 bytes starting at symbol 'data'$$
memory show $array+2 0x08          $$show 8 bytes with offset$$
memory show #0x2000 0x20 $$cache$$ $$show 32 bytes with description$$
```

You can specify a custom width (bytes per line) as an optional parameter:

```
memory show #0x1000 0x20 8         $$show 32 bytes with 8 bytes per line$$
memory show $data 0x40 4 $$table$$ $$show 64 bytes with 4 bytes per line$$
```

The output is formatted as a hex dump with both hex values and ASCII representation (where printable). For example:

```
📝 cache:
2000 : 48 65 6c 6c 6f 2c 20 77 6f 72 6c 64 21 00 00 00 | Hello, world!...
2010 : 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 00 | ................ 
```

With a custom width of 8 bytes per line:

```
2000 : 48 65 6c 6c 6f 2c 20 77 | Hello, w
2008 : 6f 72 6c 64 21 00 00 00 | orld!...
```

The command supports:
- Direct hex addresses or symbols with optional offsets
- Length in hex (1-4 digits)
- Optional width parameter (1-255, defaults to 16)
- Optional description in `$$description$$` format

#### memory fill

The `memory fill` command allows filling a range of memory with a specific value.

Basic syntax:
```
memory fill #0x1000~#0x1FFF 0x42    $$fill range with 0x42$$
memory fill $array~$array+0xFF 0x00  $$clear array of 256 bytes$$
```

The range is specified using start and end addresses separated by `~`. Both addresses support the standard address syntax including symbols and offsets. The fill value is optional - if omitted, the range is cleared (filled with 0x00):

```
// These are equivalent
memory fill #0x1000~#0x1FFF          $$clear range (fill with 0x00)$$
memory fill #0x1000~#0x1FFF 0x00     $$explicitly fill with 0x00$$

// Using symbols and offsets
memory fill $data~$data+0xFF         $$clear 256 bytes starting at $data$$
memory fill $array+2~$array+5 0xFF   $$fill elements 2-5 with 0xFF$$
```

Note that like all address operations, ranges wrap at the 64K boundary. For example:
```
memory fill #0xFFFF~#0x0002 0x42     $$fills 0xFFFF, 0x0000, 0x0001, 0x0002$$
```

### registers

The `registers` instructions are used to set the registers in a known state prior to testing.

#### registers flush

```
registers flush
```

Clear the registers. It sets the general registers to `0x00`, the stack pointer to `0xff` and the status to `nv-Bdizc`.
The command pointer is set to `0x0000`.

#### registers set

```
registers set A=0x01
```

Set a register to the given value.

#### registers show

The `registers show` command displays the current state of CPU registers. It can be used in two ways:

```
registers show              $$display all registers$$
registers show A            $$display only the accumulator$$
registers show cycle_count  $$display only the cycle count$$
```

When showing all registers, the output includes:
- **A** - Accumulator (8-bit) in hex and decimal
- **X** - X register (8-bit) in hex and decimal  
- **Y** - Y register (8-bit) in hex and decimal
- **S** - Status register as binary flags (NV-BdIzc format)
- **SP** - Stack pointer (8-bit) in hex and decimal
- **CP** - Command pointer/Program counter (16-bit) in hex
- **cycle_count** - Total CPU cycles executed (64-bit) in decimal

Example output:
```
🔧 Registers:
   A  = 0x42  (66)
   X  = 0xFA  (250)
   Y  = 0xB5  (181)
   S  = 0b11110100  NV-BdIzc
   SP = 0x84  (132)
   CP = 0x1005
   cycle_count = 55
```

When showing a specific register, only that register's value is displayed:
```
🔧 cycle_count = 55
🔧 A = 0x42  (66)
```

Available registers for individual display: `A`, `X`, `Y`, `S`, `SP`, `CP`, `cycle_count`.

### symbols

Symbols work exactly the same as memory addresses when dealing with memory, assert commands, and run until statements.

Whereas memory addresses are prefixed with `#0x`, symbols can be used by prefixing their name with `$`.

#### symbols load

Symbols can be loaded from files using the VICE format, for example given a symbols file `tests/symbols.txt` with:

```
al 002000 .main
al 002006 .cust_init
```

This can be loaded with:

```
symbols load "tests/symbols.txt"
```

Symbols can then be used in place of memory addresses.

With the above definitions loaded, the following are equivalent:

```
assert #0x2000=0xa9     $$first byte of code is LDA (0xa9)$$
assert $main=0xa9       $$symbol main is loaded from table$$
```

#### symbols add

Symbols can be directly add to the symbols table with:

```
symbols add RUNADL=0x02e0
symbols add RUNADH=0x02e1
```

#### Symbol byte references

When working with 16-bit addresses, you can extract the low or high byte of a symbol's address value using special syntax:

- `<$symbol` - Gets the low byte (LSB) of the symbol's address
- `>$symbol` - Gets the high byte (MSB) of the symbol's address

Examples:
```
symbols add test_str1=0x1234

// These assertions test that memory locations contain the correct address bytes
assert $cps_strings_a = <$test_str1  $$t00: cputs called with correct pointer (low byte)$$
assert $cps_strings_x = >$test_str1  $$t00: cputs called with correct pointer (high byte)$$
```

In this example:
- `<$test_str1` evaluates to `0x34` (low byte of 0x1234)
- `>$test_str1` evaluates to `0x12` (high byte of 0x1234)

This is particularly useful when testing subroutines that expect pointer arguments passed in separate registers or memory locations, which is common in 6502/65C02 programming.

#### Using symbols with registers

Symbol values also work with setting registers, they must be a single byte for registers, otherwise the command will fail.

```
symbols add SMALL=0xFF
symbols add LARGE=0x1234

// this is ok
registers set A=$SMALL

// this will fail
registers set A=$LARGE
```

### disassemble memory_start length

To output disassembly of memory location, use the command `disassemble memory_start length` where length is a hex value (1-4 digits) specifying how many bytes to disassemble.

Symbols can be used for the start address.

The soft65c02_tester enhances the base disassembler in the library by adding symbol references, and constructing both branch and standard labels, to help reading the disassembly.

Where symbols match the disassembly output addresses, they will be output as a comment, with all matching symbols displayed.

Examples:
```
disassemble $_main 0x1D
```

The output contains labels and names where they match:
```
🔍 ---- Start of disassembly ----
🔍 start, main:
🔍 #0x1000: (18)          CLC  
🔍 #0x1001: (a9 10)       LDA  #$10
🔍 #0x1003: (6d 00 20)    ADC  $2000         ; → mem_lo
🔍 #0x1006: (8d 00 20)    STA  $2000         ; → mem_lo
🔍 #0x1009: (90 0d)       BCC  branch_1
🔍 #0x100B: (ee 01 20)    INC  $2001         ; → mem_hi
🔍 #0x100E: (d0 05)       BNE  branch_2
🔍 #0x1010: (f0 ee)       BEQ  start         ; → start, main
🔍 #0x1012: (4c 1b 10)    JMP  $101B         ; → end
🔍 branch_2:
🔍 #0x1015: (a9 00)       LDA  #$00
🔍 #0x1017: (60)          RTS  
🔍 branch_1:
🔍 #0x1018: (a9 ff)       LDA  #$ff
🔍 #0x101A: (60)          RTS  
🔍 end:
🔍 #0x101B: (a9 42)       LDA  #$42
🔍 #0x101D: (60)          RTS  
🔍 ----- End of disassembly -----
```

### run

#### running step by step

The `run` keyword performs execution of instructions that modify the state of the memory and/or the registers. When used alone, the instruction in the memory pointed at from the `CP` register is executed. 

```
registers flush
registers set CP=0x1000
memory write #0x1000 0x(a9,00)
run
```

The script above, when ran with the `verbose` parameter, will output the following:

```
🔧 Setup: registers flushed
🔧 Setup: register CP set to #0x1000
🔧 Setup: 2 bytes written
🚀 #0x1000: (a9 00)       LDA  #$00     (#0x1001)  [A=0x00][S=nv-BdiZc][2]
```

It is also possible to change the `CP` register prior to execution directly. The example above then becomes:

```
registers flush
memory write #0x1000 0x(a9,00)
run #0x1000
```

If the execution aims at testing the execution at boot time, it is possible to follow the `init` vector contained at memory address `0xfffc-d`:

```
registers flush
memory write #0x1000 0x(a9,00)
memory write #0xfffc 0x(00,10)
run init
```

#### run until a condition is met

Sometimes, tests require running a lot of instructions before conditions are met to actually perform tests. It is possible to launch an execution until a given condition is met.

```
registers flush
// actual program: LDA #?c0; TAX; TAY
memory write #0x1000 0x(a9,c0,aa,a8)
run #0x1000 until X!=0
```

The example below outputs the following lines:

```
🔧 Setup: registers flushed
🔧 Setup: 4 bytes written
🚀 #0x1000: (a9 c0)       LDA  #$c0     (#0x1001)  [A=0x00][S=Nv-Bdizc][2]
🚀 #0x1002: (aa)          TAX                      [X=0x00][S=Nv-Bdizc][2]
```

One noticed that the third instruction `TAY` is not executed since the execution stops after `0xaa TAX` set the X register to `0xc0`. The condition is evaluated **after** each instruction is executed.

It is possible to specify conditions about any register or memory location:

```
// run until a memory location value changes
run until #0x00a1 != 0x00

// run forever
run until false
```

Note that in all cases, the execution will stop if the command pointer register has not changed after an instruction to prevent dummy infinite loops or when the `STP` instruction is met.

#### run while a condition is true

In addition to running until a condition is met, you can run while a condition remains true:

```
// Run as long as X is less than 0x80
run while X < 0x80

// Run as long as a complex condition is true
run while (A = 0x42 AND X < 0x10) OR Y > 0x20
```

The condition is checked before each instruction. If the condition is false, execution stops immediately without executing the next instruction.

### cycle timing

The emulator accurately tracks CPU cycle timing through the `cycle_count` register. This is a 64-bit counter that tracks the total number of cycles executed by the CPU. Each instruction consumes a specific number of cycles based on:

- The base instruction timing
- Additional cycles for page boundary crossings in indexed addressing modes
- Extra cycles for decimal mode arithmetic on the 65C02
- Branch instruction timing (additional cycle when taken, and another when crossing page boundary)

The cycle count can be:
- Reset using `registers set cycle_count=0`
- Used as a condition in `run while` statements (e.g., `run while cycle_count < 256`)
- Compared against decimal or hex values (e.g., `assert cycle_count = 42` or `assert cycle_count = 0x2A`)

Example usage:
```
registers set cycle_count=0             // reset cycle counter
run while cycle_count < 256             // run until 256 cycles have passed
assert cycle_count >= 0x100             // verify minimum cycles executed
```

Note that when using cycle_count in a `run while` condition, the execution will continue until the condition is checked after completing the current instruction. This means the final cycle count may be slightly higher than the specified value due to multi-cycle instructions.

### Assertions

Testing is about assertions. The `assert` keyword ensure the given condition is met during the execution process. It is possible to check conditions for memory or registers.

```
assert #0x1234 > 0x7f   $$checking memory value$$
assert A <= 0x1e        $$checking accumulator value$$
```

When the expectations are not met, an error is thrown and the rest of the execution plan is ignored (see [`marker`](###marker) above).

Each assertion has a text description that is displayed when evaluated. 

```
assert false    $$this assertion always fails$$
assert true     $$although always ok, this assertion is not evaluated$$
```

#### Logical Operators

Conditions can be combined using logical operators AND, OR, and NOT:

```
assert A = 0x42 AND X = 0x10   $$both conditions must be true$$
assert A = 0x42 OR X = 0x10    $$at least one condition must be true$$
assert NOT A = 0x42            $$condition must be false$$
```

Complex expressions with parentheses are supported:

```
assert (A = 0x42 AND X < 0x10) OR Y > 0x20   $$complex condition$$
assert NOT (A = 0x42 AND X = 0x10)           $$negation of a compound condition$$
```

#### asserting memory values

You can assert that a memory location contains a specific value:
```
assert $counter = 0x42  $$verify counter value$$
```

Memory locations can include offsets (see [Address Offsets](#address-offsets) section):
```
assert $array + 1 = 0x00  $$verify second element is zero$$
assert $array - 1 = 0xFF  $$verify previous element is 0xFF$$
```

You can also verify sequences of bytes in memory:
```
assert $data ~ 0x(01,02,03)  $$verify three bytes in sequence$$
```

If an array fails to match the expected bytes, the corresponding memory is shown with a hex dump like this:

```
⚡ 06 → t2: first 8 bytes of pagegroup data ❌ ((#0x3B1A ~ 0x(01,01,02,03,04,05,06,07))
Memory comparison failed:

Expected:
3B1A : 01 01 02 03 04 05 06 07                         | ........

Actual:
3B1A : 00 01 02 03 04 05 06 07                         | ........
)
```

#### asserting pointers

The `->` operator can be used to verify that a memory location contains a pointer (16-bit address in little-endian format) that points to a specific target address. This is particularly useful for testing indirect addressing and pointer manipulation.

Basic syntax:
```
assert $pointer -> $target  $$verify pointer points to target$$
```

Pointers support the same offset syntax as memory addresses (see [Address Offsets](#address-offsets) section):
```
assert $entry_loc -> $cache + 0x20   $$entry_loc should point to second cache entry$$
assert $stack_ptr -> $stack_top - 32 $$stack_ptr should point 32 bytes below stack top$$
```

This verifies that:
1. The low byte at `$entry_loc` contains the correct low byte of the target address
2. The high byte at `$entry_loc + 1` contains the correct high byte of the target address

Note that pointer arithmetic wraps around at 16 bits, following 6502 behavior:
```
assert $ptr -> $near_end + 0x30  $$pointer wraps from 0xFFE0 + 0x30 to 0x0010$$
assert $ptr -> $near_start - 0x30  $$pointer wraps from 0x0020 - 0x30 to 0xFFF0$$
```

### Strings

String literals are supported in both memory write, and assertions on byte sequences.

Examples can be seen in the [test atari binary script](tests/test_atari.txt)

```
// equivalent writes with standard escape sequences
memory write #0x1100 "abc\n\0def"
memory write #0x1100 0x(61,62,63,0a,00,64,65,66)

// using hex escape sequences for more control
memory write #0x1200 "data:\x0A\xFF\x00end"
memory write #0x1200 0x(64,61,74,61,3a,0a,ff,00,65,6e,64)

// using line continuation for multi-line layouts
memory write #0x1300 "\
+-------+\
| Hello |\
+-------+"

// equivalent assertions
assert #0x1100 ~ "abc\n\0def"  $$string matches at location 0x1100 with string comparison$$
assert #0x1100 ~ 0x(61,62,63,0a,00,64,65,66)  $$string matches at location 0x1100 with bytes comparison$$
assert #0x1200 ~ "data:\x0A\xFF\x00end"  $$string with hex escapes matches$$
assert #0x1300 ~ "+-------+| Hello |+-------+"  $$multi-line string matches$$
```

### Value Formats

Values can be specified in several formats:

#### Hexadecimal Values

Hexadecimal values can be written in both long and short forms:
```
assert A = 0x0F   // long form
assert A = 0xF    // short form - single digit is allowed
```

This applies to all contexts where hex values are used:
```
memory write #0x1234 0x(F,A,C)   // short form in byte sequences
registers set A=0xF              // short form in register assignment
```

#### Decimal Values

Values can also be specified in decimal format:
```
assert A = 42        // decimal value
assert X >= 128      // decimal comparison
run while A < 200    // decimal in conditions
```

This works in any context where a value is expected:
```
registers set A=42              // decimal in register assignment
run until cycle_count >= 256    // decimal in cycle count comparison
```

### Memory and Address Handling

#### Memory Addresses

Memory addresses can be specified in two ways:
1. Direct hexadecimal addresses: `#0x1234`
2. Symbol references: `$symbol_name`

#### Address Offsets

Both memory addresses and pointer targets can use offsets for array-like access. Offsets can be:
- Positive: `address + value`
- Negative: `address - value`
- In hexadecimal: `+ 0xFF` or `- 0x10`
- In decimal: `+ 42` or `- 32`

The offset arithmetic follows 6502 behavior, wrapping around at 16 bits (64K boundary).

Examples:
```
// Memory access with offsets
assert $array + 1 = 0x00      $$verify second element is zero$$
assert $array - 1 = 0xFF      $$verify previous element is 0xFF$$
memory write $array + 0x10 0x(42)  $$write to array with offset$$

// Memory sequences with offsets
assert $data + 2 ~ 0x(04,05)  $$verify bytes with positive offset$$
assert $data - 2 ~ 0x(FF,FE)  $$verify bytes with negative offset$$

// Pointer assertions with offsets
assert $ptr -> $base + 0x20   $$verify pointer with positive offset$$
assert $ptr -> $base - 32     $$verify pointer with negative offset$$
```

Note that all address arithmetic wraps at 64K. For example:
```
assert $0xFFFF + 2 = 0x42     $$0xFFFF + 2 wraps to 0x0001$$
assert $0x0000 - 1 = 0x42     $$0x0000 - 1 wraps to 0xFFFF$$
assert $ptr -> #0xFFFF + 0x10 $$pointer wraps from 0xFFFF + 0x10 to 0x000F$$
```

## Examples

```shell
$ cd soft65c02_tester
$ cargo build
$ ../target/debug/soft65c02_tester -v -i tests/test_atari.txt
📄 loading atari binaries
🔧 3 segments loaded.
🔧 2 symbols loaded
🔧 Symbol RUNADL added with value 0x02E0
🔧 Symbol RUNADH added with value 0x02E1
🔧 Symbol INITADL added with value 0x02E2
🔧 Symbol INITADH added with value 0x02E3
🔧 Symbol COLOR1 added with value 0x02C5
🔧 Symbol COLOR2 added with value 0x02C6
🔧 Symbol COLOR3 added with value 0x02C7
🔧 Symbol COLOR4 added with value 0x02C8
🔧 registers flushed
⚡ 01 → RUNADR = 0x2000 low byte ✅
⚡ 02 → RUNADR = 0x2000 high byte ✅
⚡ 03 → INITADR = 0x2006 low byte ✅
⚡ 04 → INITADR = 0x2006 high byte ✅
⚡ 05 → first byte of code is LDA (0xa9) ✅
⚡ 06 → symbol main is loaded from table ✅
⚡ 07 → 0x2000 starts with correct byte sequence ✅
⚡ 08 → main starts with correct byte sequence ✅
🔍 ---- Start of disassembly ----
🔍 main:
🔍 #0x2000: (a9 42)       LDA  #$42
🔍 #0x2002: (8d c6 02)    STA  COLOR2
🔍 #0x2005: (60)          RTS  
🔍 cust_init:
🔍 #0x2006: (a2 00)       LDX  #$00
🔍 #0x2008: (8e c8 02)    STX  COLOR4
🔍 #0x200B: (60)          RTS  
🔍 #0x200C: (00)          BRK  
🔍 ----- End of disassembly -----
🚀 #0x2000: (a9 42)       LDA  #$42     (#0x2001)  [A=0x42][S=nv-Bdizc][2]
⚡ 09 → A is $42 ✅
⚡ 10 → Target location is 0 before changed ✅
🚀 #0x2002: (8d c6 02)    STA  $02C6    (#0x02C6)  (0x42)[4]
⚡ 11 → Changes to value in A ✅
🚀 #0x2005: (60)          RTS                      [CP=0x0001][6]
⚡ 12 → Exit function ✅
🔧 Setup: register X set to 0xff
🔧 Setup: 1 byte written
🚀 #0x2006: (a2 00)       LDX  #$00     (#0x2007)  [X=0x00][S=nv-BdiZc][2]
🚀 #0x2008: (8e c8 02)    STX  $02C8    (#0x02C8)  0x00[S=nv-BdiZc][4]
🕒 Total cycles: 6
⚡ 13 → X is set to 00 ✅
⚡ 14 → Changes to value in X ✅
🚀 #0x200B: (60)          RTS                      [CP=0x0001][SP=0x03][S=nv-BdiZc][6]
⚡ 15 → Exit function ✅
🔧 Setup: 5 bytes written
⚡ 16 → string "hello" is at location 0x1000 ✅
🔧 Setup: 8 bytes written
⚡ 17 → string matches at location 0x1100 with string comparison ✅
⚡ 18 → string matches at location 0x1100 with bytes comparison ✅
```
