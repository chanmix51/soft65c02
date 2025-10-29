        .export     __STARTUP__ : absolute = 1
        .export     start
        .export     _halt

        .import     _main
        .import     __MAIN_SIZE__, __MAIN_START__
        .import     __STACKSIZE__

        .include    "zeropage.inc"

; this is deliberately over complicated for the example of gfx_demos, but can
; be used directly for full C applications loaded into the emulator after compiling with cc65

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

        ; FFFC is the init vector
        lda     #<start
        sta     $FFFC
        lda     #>start
        sta     $FFFD

        ; reserve space for software stack. This is only really required if you have C apps using it
        ; but doesn't hurt to leave in for pure asm cases.
        lda     #<(__MAIN_START__ + __MAIN_SIZE__ + __STACKSIZE__)
        ldx     #>(__MAIN_START__ + __MAIN_SIZE__ + __STACKSIZE__)
        sta     sp
        stx     sp+1

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
        .byte   $db         ; STP in 65c02 emulator (instruction 'STP' isn't supported in cc65 without flags)
