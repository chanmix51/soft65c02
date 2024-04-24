## Soft65C02 Tester

The tester can run machine code, write to memory or registers and execute
assertions. Here is a example of test script:

```
marker                      $$simple test$$
// useful comment
memory load #0x8000 "whatever.bin"
assert #0xfffc=0x00         $$low byte of init vector is 0x00$$
assert #0xfffd=0x80         $$high byte of init vector is 0x80$$

run init until CP=0x8005    $$run the init vector until the first subroutine$$
assert A=0x00               $$accumulator is initialized to 0x00$$
assert X=0x00               $$register X is initialized to 0x00$$
assert Y=0x00               $$register Y is initialized to 0x00$$
assert S=0xfe               $$stack pointer is initialized to 0xfe$$

marker                      $$another test plan$$
memory load #0x8000 "../whatever.bin"
registers set A=0xff
run #0x80ad until CP=0x80b0 $$run the pika subroutine$$
assert A!=0x00              $$accumulator is loaded with a non black pixel value$$
```

Here is the output of the tests:

```
📄 simple test
⚡ 01 → low byte of init vector is 0x00 ✅
⚡ 02 → high byte of init vector is 0x80 ✅
⚡ 03 → accumulator is initialized to 0x00 ✅
⚡ 04 → register X is initialized to 0x00 ✅
⚡ 05 → register Y is initialized to 0x00 ✅
⚡ 06 → stack pointer is initialized to 0xfe ✅
📄 another test plan
⚡ 07 → accumulator is loaded with a non black pixel value ❌
Error: Assertion failed
```

When debugging tests, it may be useful to use the `verbose` option:

```
📄 simple test
🔧 Setup: 32768 bytes loaded from 'whatever.bin' at #0x8000.
⚡ 01 → low byte of init vector is 0x00 ✅
⚡ 02 → high byte of init vector is 0x80 ✅
🚀 #0x0000
⚡ 03 → accumulator is initialized to 0x00 ✅
⚡ 04 → register X is initialized to 0x00 ✅
⚡ 05 → register Y is initialized to 0x00 ✅
⚡ 06 → stack pointer is initialized to 0xfe ✅
📄 another test plan
⚡ 07 → accumulator is loaded with a non black pixel value ❌
Error: Assertion failed
```
