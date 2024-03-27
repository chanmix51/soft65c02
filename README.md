Soft 65C02 ==========

Broken for now for heavy rework.  The last working version is e7e6615

![GNU GPL V3.0](https://img.shields.io/github/license/chanmix51/soft65c02)
![Rust Language](https://img.shields.io/badge/language-rust-orange)

Soft 65C02 is:

 * a 65C02 emulator library that can:
    * disassemble or run 65C02 instructions
    * propose a modular memory layout (RAM, ROM and video framebuffer limited
      support for now)

The future version will also ship:
 * a command line tool based on top of the library that can interactively:
    * load binary 65C02 code into memory
    * check any part of the memory
    * dump registers content at any point
    * disassemble portions of the memory
    * run programs until conditional expressions on registers or memory are met
    * run programs step by step
    * please you as much as readline can (with some auto completion)
    * kill infinite loops by pressing CTRL-C
    * help you with documentation and (hopefully) meaningful error messages
 * a testing tool for compiled binaries that can
    * do assertions on registers and memory
    * write bytes in memory (to simulate I/Os)
    * take scripts on its standard input

The library is heavily tested, lot of the parts were coded driven by tests so
it might be sort of reliable.  The addressing mode mechanisms and operands were
tested against [Klaus Dormann's 6502/65C02 test
suite](https://github.com/Klaus2m5/6502_65C02_functional_tests/blob/master/65C02_extended_opcodes_test.a65c)
and it looks like it ended up with succeeding both tests. Lot of informations
about the hidden secrets of these processors were found on the [6502.org
website](http://www.6502.org/) which is a gold mine crafted with patience by
passionate people, thanks a lot to them for the wonderful tutos and
documentation they wrote and shared.

This is a pet project so if you are interested into real world software
regarding this processor, I suggest you have a look
[there](https://www.masswerk.at/products.php) and
[there](http://www.6502.org/users/andre/).

Why would you write another simulator for the 65C02?
---------------------------------------------------- To learn the [Rust
language](https://www.rust-lang.org/) and … the 65C02. It has been a real
pleasure to code in the Rust programming language, guys & girls, you have
created awesome tools.

The Rust language OK, but why the 6502 and not a modern processor?
------------------------------------------------------------------ Because the
6502 comes from an age where processors were built by humans for humans. So the
6502 is a very good way to learn how processors work and are programmed. Plus,
the 65C02 is cool. It's all [Ben
Eater](https://www.youtube.com/watch?v=LnzuMJLZRdU)'s fault by the way.

What's the actual state of this development?
-------------------------------------------- The library is supposed to be wire
to wire compatible with a real 65C02. The CLI and the asserter from the
previous version have been removed and a new version will be hopefully
a better experience as it was.

License ------- This software is released under the terms of the [GNU GPL
v3](http://www.gnu.org/licenses/gpl-3.0.html). In short, you are free to do
what you want with this software but the author is NOT responsible _in any
case_ for whatever happens directly or indirectly as results of your use of the
software. You are able to modify and/or share the software as long as it, with
your changes, remains under the same license.

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE.  See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program.  If not, see [https://www.gnu.org/licenses/].
