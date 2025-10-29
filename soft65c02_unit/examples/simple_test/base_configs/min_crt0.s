        .export     __STARTUP__ : absolute = 1
        .export     start
        .export     _halt

        .import     _main

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
