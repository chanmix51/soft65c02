Soft 65C02
==========
![travis-CI](https://api.travis-ci.org/chanmix51/soft65c02.svg?branch=master)

Soft 65C02 is yet another 65C02 processor simulator. Its purpose is to be:

 * reliable: it is heavily tested, it has been driven by tests.
 * able to disassemle AND also execute binary code
 * flexible: its memory can be configured at will (RAM, ROM, video etc.)
 * interactive, the user (me) has to be able through a CLI to:
   * execute code step by step or until a certain condition is reached or full run.
   * disassemble any given part of the memory at any point
   * view any given part of the memory at any point
   * dump registers state at any point

It is a (maybe yet another unfinished) pet project so if you are interested into real world software regarding this processor, I suggest you have a look [there](https://www.masswerk.at/products.php) and [there](http://www.6502.org/users/andre/).

Why would you write another simulator for the 65C02?
----------------------------------------------------
To learn the Rust language and … the 65C02.

The Rust language OK, but why the 6502 and not a modern processor?
------------------------------------------------------------------
Because the 6502 comes from an age where processors were built by humans for humans so the 6502 is a very good way to learn how processors work and are programmed. Plus, the 65C02 is cool. It's all [Ben Eater](https://www.youtube.com/watch?v=LnzuMJLZRdU)'s fault by the way.

What this simulator is supposed to do?
--------------------------------------

 * binary file loader ✓
 * code disassembler ✓
 * step by step execution ✓
 * breakpoints & conditional debugger ✗
 * memory & registers explorer ✓
 * interactive interface ✗
 * aims at being the more modular possible to be able to plug virtual devices like screen (through minifb) I/O devices etc. (video ok, kb at some point)

What's the actual state of this development?
--------------------------------------------
Experimental work in progress. Do not expect anything from it.

What is working right now?
It is possible to load and run programs assembled for the 6502. It is possible to disassemble part of it and dump part of the memory.

The following part of the ASM program:

```asm
    wait:
        NOP
        STZ     SCREEN_START
        LDA     ADDR_KEYBOARD ; read keyboard
        BEQ     wait          ; nothing?
        STA     ADDR_VAR_KB   ; store the key pressed
        LDA     #KEY_J
        SBC     ADDR_VAR_KB   ; compare it with J
        BNE     key_l         ; is it not J ?
        LDA     #$0           ; paint black
        STA     ($00),Y       ; at actual video position
        DEY                   ; move left
        BNE     draw
```

once compiled and loaded through soft65C02:

```rust
    let init_vector:usize = 0x1B00;
    let mut memory = Memory::new_with_ram();
    let mut f = File::open("point.bin").unwrap();
    let mut buffer:Vec<u8> = vec![];
    f.read_to_end(&mut buffer).unwrap();
    let len = buffer.len();
    memory.write(init_vector, buffer).unwrap();
    for line in soft65c02::disassemble(init_vector, init_vector + len, &memory).iter() {
        println!("{}", line);
    }
```

will produce the following output:

    #0x1B12: (ea)          NOP
    #0x1B13: (9c 00 03)    STZ  $0300
    #0x1B16: (ad 30 02)    LDA  $0230
    #0x1B19: (f0 f7)       BEQ  -9
    #0x1B1B: (85 03)       STA  $03
    #0x1B1D: (a9 6a)       LDA  #$6a
    #0x1B1F: (e5 03)       SBC  $03
    #0x1B21: (d0 0f)       BNE  +15
    #0x1B23: (a9 00)       LDA  #$00
    #0x1B25: (91 00)       STA  ($00),Y
    #0x1B27: (88)          DEY
    #0x1B28: (d0 e4)       BNE  -28

It is also possible to run it step by step or not:

    let mut registers = Registers::new(init_vector);
    let mut cp = 0x0000;

    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        println!("{}", soft65c02::execute_step(&mut registers, &mut memory).unwrap());
        thread::sleep(time::Duration::from_millis(10));
    }

this will output something like:

    #0x1B12: (ea)          NOP
    #0x1B13: (9c 00 03)    STZ  $0300    (#0x0300)
    #0x1B16: (ad 30 02)    LDA  $0230    (#0x0230)  [A=0x00][S=nv-BdiZc]
    #0x1B19: (f0 f7)       BEQ  -9       (#0x1B12)  [CP=0x1B12]
    #0x1B12: (ea)          NOP
    #0x1B13: (9c 00 03)    STZ  $0300    (#0x0300)
    #0x1B16: (ad 30 02)    LDA  $0230    (#0x0230)  [A=0x00][S=nv-BdiZc]
    #0x1B19: (f0 f7)       BEQ  -9       (#0x1B12)  [CP=0x1B12]
    #0x1B12: (ea)          NOP
    #0x1B13: (9c 00 03)    STZ  $0300    (#0x0300)
    #0x1B16: (ad 30 02)    LDA  $0230    (#0x0230)  [A=0x6a][S=nv-Bdizc]
    #0x1B19: (f0 f7)       BEQ  -9       (#0x1B12)  [CP=0x1B1B]
    #0x1B1B: (85 03)       STA  $03      (#0x0003)
    #0x1B1D: (a9 6a)       LDA  #$6a     (#0x1B1E)  [A=0x6a][S=nv-Bdizc]
    #0x1B1F: (e5 03)       SBC  $03      (#0x0003)  (0x6a)[A=0x00][S=nv-BdiZc]
    #0x1B21: (d0 0f)       BNE  +15      (#0x1B32)  [CP=0x1B23]
    #0x1B23: (a9 00)       LDA  #$00     (#0x1B24)  [A=0x00][S=nv-BdiZc]
    #0x1B25: (91 00)       STA  ($00),Y  (#0x0C20)
    #0x1B27: (88)          DEY                      [Y=0x1f][S=nv-Bdizc]
    #0x1B28: (d0 e4)       BNE  -28      (#0x1B0E)  [CP=0x1B0E]

It has limited but functional support of [rust minifb](https://github.com/emoon/rust_minifb) which makes this emulator a computer with a graphical (32bits) screen and a keyboard.

Work in progress
----------------

Soft65C02 now recognizes all 65C02 opcodes but BBRx & RMBx and decimal mode operations. It passes part of the [6502 functional testing](https://github.com/Klaus2m5/6502_65C02_functional_tests), it sounds there is still a bug in the SBC instruction. Investigation will take place later. Now I am working on the user interface, a CLI using Rustyline for readline support and [Pest](https://github.com/pest-parser/pest) for grammar parsing. If I do it, it should make a decent interactive parser & debugger.
