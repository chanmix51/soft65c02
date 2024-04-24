# Soft65C02 Library

![GNU GPL V3.0](https://img.shields.io/github/license/chanmix51/soft65c02)
![Rust Language](https://img.shields.io/badge/language-rust-orange)

The library provides a set of Rust structures to perform step by step execution of CPU code.

The library is heavily tested, lot of the parts were coded driven by tests so
it might be sort of reliable.  The addressing mode mechanisms and operands were
tested against [Klaus Dormann's 6502/65C02 test
suite](https://github.com/Klaus2m5/6502_65C02_functional_tests/blob/master/65C02_extended_opcodes_test.a65c)
and it looks like it ended up with succeeding both tests. Lot of informations
about the hidden secrets of these processors were found on the [6502.org
website](http://www.6502.org/) which is a gold mine crafted with patience by
passionate people, thanks a lot to them for the wonderful tutos and
documentation they wrote and shared.

### known limitations

The library does not take the real 65C02 cycles in account. If you have to rely
on real clock ticks it will not help you.

