    .export     _main
    .import     _fn_under_test

; this will setup any appropriate data required for the function and then run it.
; for this simple test, there's no parameters or anything to setup, so just call it.
_main:
    jsr     _fn_under_test
    rts
