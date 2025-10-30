        .export     __STARTUP__ : absolute = 1
        .export     halt

; this is the beginning of the app, if anything directly calls it, we halt
.segment "STARTUP"
halt:
        .byte   $db         ; STP in 65c02 emulator

; if "init" is called in soft65c02_tester, call halt and stop the emulator
.segment "V_RESET"
        .word halt
