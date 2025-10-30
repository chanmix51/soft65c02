        .export     fn_under_test

        ; export symbols that will be used in the test script
        .export     my_loc, a_save, x_save, y_save, my_loc_low, my_loc_hi
        .export     x_loop

a_save     = $80
x_save     = $81
y_save     = $82
my_loc_low = $83
my_loc_hi  = $84

fn_under_test:
        ; save registers for testing
        sta     $80
        stx     $81
        sty     $82

        ; now do some code that causes the counter to increment for a while, so we can test the cycle count
        ; the loop will write 
        lda     #<my_loc
        sta     $83
        lda     #>my_loc
        sta     $84

        ldy     #$00
        ldx     #$10
        lda     $80
        clc
x_loop:
        sta     ($83), y
        adc     #$01           ; increment A
        iny
        dex
        bne     x_loop
        rts

my_loc:
        .word $00
