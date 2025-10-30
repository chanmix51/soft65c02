        .export     __STARTUP__ : absolute = 1
        .export     start
        .export     _halt

        .import     _main

        ; required for c_sp. Note, this needs latest versions of cc65 to work, as the old "sp" name is deprecated.
        .include    "zeropage.inc"

.segment "STARTUP"
start:
        ; set INTERRUPT/NMI vectors to a _halt address, which issues a STP
        ; this will cause any "BRK" in the application to call "_halt", and thus stop the emulator
        lda     #<_halt
        sta     $FFFE   ; INTERRUPT
        sta     $FFFA   ; NMI
        lda     #>_halt
        sta     $FFFF   ; INTERRUPT
        sta     $FFFB   ; NMI

        ; Set the CC65 software stack to somewhere out of the way.
        ; This isn't needed if you're just running ASM code and not C code, but it doesn't hurt to keep it.
        lda     #<$F000
        ldx     #>$F000
        sta     c_sp
        stx     c_sp+1

        ; setup stack pointer to something sensible
        ldx     #$ff
        txs

        ; clean up a bit for start of application
        inx             ; X = 0
        txa
        tay
        clc

        ; call main
        jmp     _main


_halt:
        .byte   $db         ; STP in 65c02 emulator

; this will cause the emulator to set the address "init" to the "start" address, so you can use "run init"
.segment "V_RESET"
        .word start
