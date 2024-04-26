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

#### memory write

```
memory write #0x1234 0x(00,01,02,…)
```

Write a slice of contiguous bytes at the given address.

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
🚀 #0x1000: (a9 00)       LDA  #$00     (#0x1001)  [A=0x00][S=nv-BdiZc]
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
🚀 #0x1000: (a9 c0)       LDA  #$c0     (#0x1001)  [A=0x00][S=Nv-Bdizc]
🚀 #0x1002: (aa)          TAX                      [X=0x00][S=Nv-Bdizc]
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