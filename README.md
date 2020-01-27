Soft 65C02
==========
![travis-CI](https://api.travis-ci.org/chanmix51/soft65c02.svg?branch=master)

Soft 65C02 is yet another 65C02 processor simulator. Its purpose is to be:

 * reliable: it is heavily tested, it has been driven by tests.
 * able to disassemble and also execute binary code.
 * flexible: its memory can be configured at will (RAM, ROM, video etc.)
 * interactive: it is possible through a CLI to:
   * execute code step by step or until a certain condition is reached or full run
   * disassemble any given part of the memory at any point
   * view any given part of the memory at any point
   * dump registers state at any point
   * break infinite loops by pressing CTRL-C

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
 * breakpoints & conditional debugger ✓
 * memory & registers explorer ✓
 * interactive interface ✓
 * aims at being the more modular possible to be able to plug virtual devices like screen (through minifb) I/O devices etc. (video ok, keyboard at some point).

What's the actual state of this development?
--------------------------------------------
Experimental work in progress. The CLI mode is now working almost properly:

 * readline support (history, auto completion etc.) thank to the Rusty library
 * commands and execution boolean condition, thanks to the [Pest](https://github.com/pest-parser/pest) parser
 * it is possible to:
    * load binary file into memory
    * disassemble or run program with breaking conditions or not
    * dump memory or register content
    * ctrl-c breaks a running program but does not exit the CLI

Work in progress
----------------

Soft65C02 recognizes all 65C02 opcodes but BBRx & RMBx and also lacks decimal mode operations. It passes part of the [6502 functional testing](https://github.com/Klaus2m5/6502_65C02_functional_tests), it sounds there is still a bug in the SBC instruction (test $29 is failing for now).

It might be a good idea to be able to send interrupts through the CLI. 
