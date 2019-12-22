Soft 65C02
==========

Soft 65C02 is yet another 65C02 processor simulator. If you are interested into real world software regarding this processor, I suggest you have a look [there](https://www.masswerk.at/products.php) and [there](http://www.6502.org/users/andre/).

Why would you write another simulator for the 65C02?
----------------------------------------------------
To learn the Rust language and … the 65C02.

The Rust language OK, but why the 6502 and not a modern processor?
------------------------------------------------------------------
Because the 6502 comes from an age where processors were built by humans for humans so the 6502 is a very good way to learn how processors work and are programmed. Plus, the 65C02 is cool. It's all the fault of [Ben Heaters](https://www.youtube.com/watch?v=LnzuMJLZRdU) by the way.

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

Error handling should be done properly in the adressing mode resolution mechanisme. Today it just panics when an expected address is not given by the resolever but in some cases, by example the Relative addressing mode may perform an overflow and not give any results. So the Microcode shall then return an Error and the user be notified something went wrong in place of the application crash.

