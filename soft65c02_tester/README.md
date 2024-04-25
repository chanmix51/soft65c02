## Soft65C02 Tester

The tester can run machine code, write to memory or registers and execute
assertions. Here is a example of test script:

``` program.asm
.ORG $8000
LDX #$FE
TXS
LDA #$0
TAX
TAY
STP

.ORG $FFFC
.DBYT $0080,$00FF
```

```
marker                      $$testing initialization sequence$$
// load program in memory
memory load #0x8000 "whatever.bin"
// check init vector
assert #0xfffc=0x00           $$low byte of init vector is set$$
assert #0xfffd=0x80           $$high byte of init vector is set$$

// set registers to wrong state to ensure initialization works
registers set A=0xff
registers set X=0xff
registers set Y=0xff
registers set SP=0xff
registers set CP=0x0000

// run and check initialization
run init until CP=0x8007    $$run the init vector until end of initialization$$
assert A=0x00               $$accumulator is initialized$$
assert X=0x00               $$register X is initialized$$
assert Y=0x00               $$register Y is initialized$$
assert SP=0xfe              $$stack pointer is initialized$$

assert false                $$failing test$$
assert true                 $$will not be executed$$
```

Here is the output of the tests:

```
📄 test initialization
⚡ 01 → low byte of init vector is set ✅
⚡ 02 → high byte of init vector is set ✅
⚡ 03 → accumulator is initialized ✅
⚡ 04 → register X is initialized ✅
⚡ 05 → register Y is initialized ✅
⚡ 06 → stack pointer is initialized ✅
⚡ 07 → failing test ❌ (value is false)
Error: Assertion failed
```

When debugging tests, it may be useful to use the `verbose` option:

```
📄 test initialization
🔧 Setup: 32768 bytes loaded from 'tests/whatever.bin' at #0x8000.
⚡ 01 → low byte of init vector is set ✅
⚡ 02 → high byte of init vector is set ✅
🔧 Setup: register A set to 0xff
🔧 Setup: register X set to 0xff
🔧 Setup: register Y set to 0xff
🔧 Setup: register SP set to 0xff
🔧 Setup: register CP set to #0x0000
🚀 #0x8000: (a2 fe)       LDX  #$fe     (#0x8001)  [X=0xfe][S=Nv-Bdizc]
🚀 #0x8002: (9a)          TXS                      [SP=0xfe][S=Nv-Bdizc]
🚀 #0x8003: (a9 00)       LDA  #$00     (#0x8004)  [A=0x00][S=nv-BdiZc]
🚀 #0x8005: (aa)          TAX                      [X=0x00][S=nv-BdiZc]
🚀 #0x8006: (a8)          TAY                      [Y=0x00][S=nv-BdiZc]
⚡ 03 → accumulator is initialized ✅
⚡ 04 → register X is initialized ✅
⚡ 05 → register Y is initialized ✅
⚡ 06 → stack pointer is initialized ✅
⚡ 07 → failing test ❌ (value is false)
Error: Assertion failed
```
