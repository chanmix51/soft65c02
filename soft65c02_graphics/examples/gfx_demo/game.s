        .export _main

; Game CoProcessor Runner 65C02
; This version uses Rust "DMA" acceleration for computation but
; the 6502 handles input polling and mode selection logic
;
; Controls:
;   1 = Help screen (default)
;   2 = Game of Life
;   3 = Mandelbrot set
;   4 = Space-filling curves
;   5-9 = Future games
;   P = Toggle pause (stops generation but allows mode switching and game controls)
;   All other keys (with modifier support) are passed to the active game for handling
;
; Keyboard Buffer Layout:
;   $8030 = Key code (single byte - layout-aware for symbols, layout-independent for numbers/letters)

; Memory locations  
KEYBOARD_INPUT  = $8030     ; Keyboard key code
COMMAND_ADDR    = $8040     ; Game Co-processor Command address
;; 0 = no-op, 1 = generate next iteration, 2 = process keyboard input

MODE_ADDR       = $8041     ; Game Co-processor Mode setting

; ASCII character codes (from ReceivedCharacter events)
KEY_0           = $30       ; '0' key - No-op mode (ASCII '0')
KEY_9           = $39       ; '9' key - Last numbered mode (ASCII '9')
KEY_P_UPPER     = $50       ; 'P' key - Pause toggle (ASCII 'P')
KEY_P_LOWER     = $70       ; 'p' key - Pause toggle (ASCII 'p')
; Arrow key codes (special codes from get_special_key_code)
KEY_UP          = $80       ; Up arrow
KEY_DOWN        = $81       ; Down arrow
KEY_LEFT        = $82       ; Left arrow
KEY_RIGHT       = $83       ; Right arrow

; Symbol character codes (from ReceivedCharacter events - ASCII values)
CHAR_PLUS       = $2B       ; '+' character (ASCII 43)
CHAR_MINUS      = $2D       ; '-' character (ASCII 45)
CHAR_EQUALS     = $3D       ; '=' character (ASCII 61)

_main:
        ; Initialize system
        ldx     #$00
        stx     COMMAND_ADDR    ; Clear command
        inx                     ; X = 1
        stx     MODE_ADDR       ; Set default mode to Help screen (mode 1)
        
loop:
        ; Main game loop - let the 6502 do real work!
        jsr     check_input     ; Check for keyboard input first
        
        ; Only send generate command if not paused
        lda     pause_state
        bne     loop            ; Skip generate command if paused
        
        lda     #$01            ; CMD_GENERATE
        sta     COMMAND_ADDR
        jmp     loop            ; Always loop

check_input:
        ; Read keyboard input location
        lda     KEYBOARD_INPUT
        bne     :+
        rts
        
        ; Check for pause key first (special case) - accept both upper and lower case
:       cmp     #KEY_P_UPPER
        beq     @handle_pause
        cmp     #KEY_P_LOWER
        beq     @handle_pause
        
        ; Check if key is ASCII digit '0'-'9' for mode selection
        cmp     #KEY_0
        bcc     @not_mode_key   ; Branch if key < '0' (ASCII 0x30)
        cmp     #KEY_9+1
        bcs     @not_mode_key   ; Branch if key > '9' (ASCII 0x39)
        
        ; Key is ASCII digit - convert to mode number (works for all digits 0-9)
        sec                     ; Set carry for subtraction
        sbc     #KEY_0          ; Subtract ASCII '0' to get numeric value (0-9)
        sta     MODE_ADDR       ; Store the mode directly
        
@clear_input:
        ; Clear the input event so we don't process it again
        lda     #$00
        sta     KEYBOARD_INPUT       ; Clear key code
        rts
        
@handle_pause:
        ; P key pressed - toggle pause state
        lda     pause_state
        eor     #$01
        sta     pause_state
        jmp     @clear_input    ; jump Branch always to clear input
        
@not_mode_key:
        ; Not a mode key - pass to game for processing
        ; Send keyboard processing command
        lda     #$02            ; CMD_PROCESS_KEYBOARD
        sta     COMMAND_ADDR
        bne     @clear_input

.data
pause_state:            .byte 0         ; 0 = running, 1 = paused