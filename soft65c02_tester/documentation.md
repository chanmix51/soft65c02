# Soft65C02 tester

## Tester language

The parser language description is available [here](rules.pest).

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

Additionally writing supports strings, including escaped chars `\t`, `\n`, `\r`, `\0`.

Combined with symbols, you can write memory to named addresses with following:

```
memory write $main "hello, world\0"
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

#### Using symobols with registers

Symbol values also work with setting registers, they must be a single byte for registers, otherwise the command will fail.

```
symbols add SMALL=0xFF
symbols add LARGE=0x1234

// this is ok
registers set A=$SMALL

// this will fail
registers set A=$LARGE
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

### cycle timing

Cycle times are tracked for each instructions, and are based on 65c02 timings where they differ from standard 6502, which are rare.
Additionally page boundaries and branches are taken account of in calculating the cycle times.

Each log line ends with the cycle time for that particular instruction.
If you run multiple instructions, at the end of the run, a total will be output after the execution stops. The total is not displayed if only 1 instruction is executed, as the time is output on the statement itself.

```
🚀 #0x2006: (a2 00)       LDX  #$00     (#0x2007)  [X=0x00][S=nv-BdiZc][2]
🚀 #0x2008: (8e c8 02)    STX  $02C8    (#0x02C8)  0x00[S=nv-BdiZc][4]
🕒 Total cycles: 6
```

#### resetting the cycle count

`cycle_count` is a fake register, and can be manipulated in the same way as normal registers. This can be useful if wish to capture different cycle times from particular
points in the test.

```
registers set cycle_count=0x00
```



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

#### asserting sequence of bytes

The keyword `~` can be used to match sequence of bytes for assertions.
The target can be either the standard list of bytes, or string literals. See below for more information about strings.

For example:

```
assert #0x1100 ~ 0x(61,62,63,0a,00,64,65,66)
```

### Strings

String literals are supported in both memory write, and assertions on byte sequences.

Examples can be seen in the [test atari binary script](tests/test_atari.txt)

```
// equivalent writes
memory write #0x1100 "abc\n\0def"
memory write #0x1100 0x(61,62,63,0a,00,64,65,66)

// equivalent assertions
assert #0x1100 ~ "abc\n\0def"  $$string matches at location 0x1100 with string comparison$$
assert #0x1100 ~ 0x(61,62,63,0a,00,64,65,66)  $$string matches at location 0x1100 with bytes comparison$$
```


## Examples

```shell
$ cd soft65c02_tester
$ cargo build
$ ../target/debug/soft65c02_tester -v -i tests/test_atari.txt
📄 loading atari binaries
🔧 Setup: 3 segments loaded.
🔧 Setup: 2 symbols loaded
🔧 Setup: Symbol RUNADL added with value 0x02E0
🔧 Setup: Symbol RUNADH added with value 0x02E1
🔧 Setup: Symbol INITADL added with value 0x02E2
🔧 Setup: Symbol INITADH added with value 0x02E3
🔧 Setup: registers flushed
⚡ 01 → RUNADR = 0x2000 low byte ✅
⚡ 02 → RUNADR = 0x2000 high byte ✅
⚡ 03 → INITADR = 0x2006 low byte ✅
⚡ 04 → INITADR = 0x2006 high byte ✅
⚡ 05 → first byte of code is LDA (0xa9) ✅
⚡ 06 → symbol main is loaded from table ✅
⚡ 07 → 0x2000 starts with correct byte sequence ✅
⚡ 08 → main starts with correct byte sequence ✅
🚀 #0x2000: (a9 42)       LDA  #$42     (#0x2001)  [A=0x42][S=nv-Bdizc][2]
⚡ 09 → A is $42 ✅
⚡ 10 → Target location is 0 before changed ✅
🚀 #0x2002: (8d c6 02)    STA  $02C6    (#0x02C6)  (0x42)[4]
⚡ 11 → Changes to value in A ✅
🚀 #0x2005: (60)          RTS                      [CP=0x0001][SP=0x01][S=nv-Bdizc][6]
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
