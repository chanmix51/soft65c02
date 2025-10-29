        .export     _fn_under_test
        .export     x_loop

        .import     bar
        .import     foo

        .include    "zeropage.inc"

; this is some function under test, could be any name any code
_fn_under_test:
    jsr     bar
    jsr     foo

    ldy     #$00
    lda     #<my_loc
    sta     ptr1
    lda     #>my_loc
    sta     ptr1+1
    ldx     #$10
    lda     #$8a        ; test if a literal with the same value as a symbol is affected
x_loop:
    stx     tmp1
    lda     (ptr1), y
    iny
    dex
    bne     x_loop
    
    rts

.bss
my_loc:     .res 2