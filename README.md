# Soft 65C02

Soft65C02 is a suite of softwares dedicated to 65C02 machine code testing and debugging. It is composed of 3 parts:

 * soft65C02_lib: a Rust library to simulate a 65C02 processor you can use in your own developments.
 * soft65C02_tester: a CLI tool to run automated tests against 65C02 machine code.
 * soft65C02_studio: (yet to develop) a CLI tool to run, debug machine code and create tests scripts.

If you want to use the old CLI tool, the last working version is e7e6615.

This is a pet project so if you are interested into real world software
regarding this processor, I suggest you have a look
[there](https://www.masswerk.at/products.php) and
[there](http://www.6502.org/users/andre/).


![GNU GPL V3.0](https://img.shields.io/github/license/chanmix51/soft65c02)
![Rust Language](https://img.shields.io/badge/language-rust-orange)

### Why would you write another simulator for the 65C02?

I started this project in 2020 to learn the [Rust
language](https://www.rust-lang.org/) and … the 65C02. It has been a real
pleasure to code in the Rust programming language, guys & girls, you have
created awesome tools.

### The Rust language OK, but why the 6502 and not a modern processor?

Because the 6502 comes from an age where processors were built by humans for
humans. So the 6502 is a very good way to learn how processors work and are
programmed. Plus, the 65C02 is cool.

It's all [Ben
Eater](https://www.youtube.com/watch?v=LnzuMJLZRdU)'s fault by the way.

### What's the actual state of this development?

The library is supposed to be wire to wire compatible with a real 65C02. The
first version of this software (2020 → 2024) was a CLI against the library but
if this was a good first step it was not testable enough for evolving. In 2024 I
took advantage of free time to split the project in a Rust workspace to dig more
in GUI part. I rewrote the Tester with parser tests this time. The Studio is
yet to write.

### License

These softwares are released under the terms of the [GNU GPL
v3](http://www.gnu.org/licenses/gpl-3.0.html).

In short, you are free to do what you want with this software but the author is
NOT responsible _in any case_ for whatever happens directly or indirectly as
results of your use of the software. You are able to modify and/or share the
software as long as it, with your changes, remains under the same license.

This program is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License as published by the Free Software
Foundation, either version 3 of the License, or (at your option) any later
version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE.  See the GNU General Public License for more details.

You should have received a copy of the GNU General Public License along with
this program.  If not, see [https://www.gnu.org/licenses/].
