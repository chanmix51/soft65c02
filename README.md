Soft 65C02
==========

Soft 65C02 is yet another 65C02 processor simulator. If you are interested into real world software regarding this processor, I suggest you have a look [there](https://www.masswerk.at/products.php) and [there](http://www.6502.org/users/andre/).

Why would you write another simulator for the 65C02?
----------------------------------------------------
To learn the Rust language and … the 65C02.

The Rust language OK, but why the 6502 and not a modern processor?
------------------------------------------------------------------
Because the 6502 comes from an age where processors were built by humans for humans so the 6502 is a very good way to learn how processors work and are programmed. Plus, the 65C02 is cool. It's all [Ben Eater](https://www.youtube.com/watch?v=LnzuMJLZRdU)'s fault by the way.

What this simulator is supposed to do?
--------------------------------------

 * binary file loader
 * code disassembler
 * step by step execution
 * breakpoints & conditional debugger
 * memory & registers explorer
 * REPL
 * aims at being the more modular possible to be able to plug virtual devices like screen (through minifb) I/O devices etc.

What's the actual state of this development?
--------------------------------------------
Experimental work in progress. Do not expect anything from it.

What is working right now?
It is possible to load this program at by example address 0x0800: `48 a9 01 8d 00 02 6c 00 02 95 20 a1 20 51 21 96 21 7d 01 02 f9 10 12 d0 f6` and the disassember outputs:

        #0x0800: (48)          PHA
        #0x0801: (a9 01)       LDA  #$01
        #0x0803: (8d 00 02)    STA  $0200
        #0x0806: (6c 00 02)    JMP  ($0200)
        #0x0809: (95 20)       STA  $20,X
        #0x080B: (a1 20)       LDA  ($20,X)
        #0x080D: (51 21)       EOR  ($21),Y
        #0x080F: (96 21)       STX  $21,Y
        #0x0811: (7d 01 02)    ADC  $0201,X
        #0x0814: (f9 10 12)    SBC  $1210,Y
        #0x0817: (d0 f6)       BNE  -10
        #0x0819: (00)          BRK

Each operation is unit tested, so simple oprtation might work. Arithmetic operation may not work properly and everything that uses the status register does not work correctly.

A memory stack mechanism allows to create almost all memory addressing configurations possible. 

It is possible to actually execute some code, there are still lot of opcodes / addressing modes associations missing. The MiniFB test is running the following code successfuly:

       .orig $1B00
       lda #$0f
       sta $8000
       lda #$00
       tax
    loop:
       ina
       sbc $8000
       sta $0300,X
       sta $0400,X
       sta $0500,X
       inx
       bne loop
       brk

It launches a minifb window that shows pixels as they are written in the video memory. The first lines of the execution log are:

    #0x1B00: (a9 0f)       LDA  #$0f     (#0x1B01)
    #0x1B02: (8d 00 80)    STA  $8000    (#0x8000)
    #0x1B05: (a9 00)       LDA  #$00     (#0x1B06)
    #0x1B07: (aa)          TAX
    #0x1B08: (1a)          INA
    #0x1B09: (ed 00 80)    SBC  $8000    (#0x8000)
    #0x1B0C: (9d 00 03)    STA  $0300,X  (#0x0300)
    #0x1B0F: (9d 00 04)    STA  $0400,X  (#0x0400)
    #0x1B12: (9d 00 05)    STA  $0500,X  (#0x0500)
    #0x1B15: (e8)          INX
    #0x1B16: (d0 f0)       BNE  ±$f0     (#0x1B08)
    #0x1B08: (1a)          INA
    #0x1B09: (ed 00 80)    SBC  $8000    (#0x8000)
    #0x1B0C: (9d 00 03)    STA  $0300,X  (#0x0301)


