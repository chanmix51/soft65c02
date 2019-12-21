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
