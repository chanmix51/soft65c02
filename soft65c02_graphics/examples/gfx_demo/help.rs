/*
 * Help Screen Module for Soft65C02 Graphics Demo
 * 
 * Provides text rendering capabilities for displaying help information
 * on the 128×96 @ 4-bit graphics display.
 * 
 * Features:
 * - 4x6 pixel font rendering
 * - Multi-line text support
 * - Configurable text color from 16-color palette
 * - Efficient packed pixel format (2 pixels per byte)
 */

use soft65c02_lib::{Memory, AddressableIO};

// Display constants
const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 96;
const BYTES_PER_ROW: usize = SCREEN_WIDTH / 2;  // 2 pixels per byte
const VIDEO_BUFFER_START: usize = 0x8100;

// Font constants
const FONT_WIDTH: usize = 3;
const FONT_HEIGHT: usize = 5;
const FONT_HEIGHT_WITH_DESCENDER: usize = 6;  // Characters with descenders need 6 rows
const CHAR_SPACING: usize = 1;  // 1 pixel space between characters
const LINE_SPACING: usize = 2;  // 2 pixel space between lines


/// Extract a single row of pixels from a character
/// Returns a 3-bit value representing the 3 pixels in that row
///
/// Font encoding (from font.h C code):
/// - Each character is 2 bytes encoding 5 rows of 3 pixels (plus descender flag)
/// - Bit 0 of byte1 indicates if character has descender (shifts rows down by 1)
/// - The C code extracts rows with specific bit shifts and masks
fn get_font_row(ch: u8, row: usize) -> u8 {
    if ch < 32 || ch > 127 {
        return 0;  // Invalid character
    }
    
    let index = (ch - 32) as usize;
    if index >= FONT_4X6.len() {
        return 0;
    }
    
    let byte0 = FONT_4X6[index][0];
    let byte1 = FONT_4X6[index][1];
    
    // Check if this character has a descender (bit 0 of byte1)
    let has_descender = (byte1 & 0x01) == 1;
    
    // Adjust row for descenders - if has_descender, shift all rows down by 1
    let mut line_num = row as i32;
    if has_descender {
        line_num -= 1;
    }
    
    if line_num < 0 {
        return 0;  // First row is blank for descenders
    }
    
    // Replicate the C code logic exactly
    let pixel = match line_num {
        0 => byte0 >> 4,
        1 => byte0 >> 1,
        // Split over 2 bytes - from C: (((byte0) & 0x03) << 2) | (((byte1) & 0x02))
        2 => ((byte0 & 0x03) << 2) | (byte1 & 0x02),
        3 => byte1 >> 4,
        4 => byte1 >> 1,
        _ => 0,
    };
    
    pixel & 0x0E  // Mask to 3 bits (bits 1-3, bit 0 is always 0 per C code)
}

pub struct HelpScreenState {
    frame_buffer: Vec<u8>,
    write_buffer: Vec<u8>,
}

impl HelpScreenState {
    pub fn new() -> Self {
        Self {
            frame_buffer: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT],
            write_buffer: vec![0u8; BYTES_PER_ROW * SCREEN_HEIGHT],
        }
    }
    
    /// Clear the frame buffer with a background color
    pub fn clear(&mut self, color: u8) {
        self.frame_buffer.fill(color & 0x0F);
    }
    
    /// Draw a single character at the specified position
    pub fn draw_char(&mut self, ch: u8, x: usize, y: usize, color: u8) {
        let color = color & 0x0F;  // Ensure 4-bit color
        
        // Check if character has descender to determine height
        let index = if ch >= 32 && ch <= 127 { (ch - 32) as usize } else { 0 };
        let has_descender = if index < FONT_4X6.len() {
            (FONT_4X6[index][1] & 0x01) == 1
        } else {
            false
        };
        
        let char_height = if has_descender { FONT_HEIGHT_WITH_DESCENDER } else { FONT_HEIGHT };
        
        for row in 0..char_height {
            let pixel_y = y + row;
            if pixel_y >= SCREEN_HEIGHT {
                break;
            }
            
            let row_data = get_font_row(ch, row);
            
            for col in 0..FONT_WIDTH {
                let pixel_x = x + col;
                if pixel_x >= SCREEN_WIDTH {
                    break;
                }
                
                // Check if this pixel is set in the font
                if (row_data & (1 << (3 - col))) != 0 {
                    self.frame_buffer[pixel_y * SCREEN_WIDTH + pixel_x] = color;
                }
            }
        }
    }
    
    /// Draw a string at the specified position
    pub fn draw_string(&mut self, text: &str, x: usize, y: usize, color: u8) {
        let mut cursor_x = x;
        
        for ch in text.bytes() {
            if cursor_x + FONT_WIDTH > SCREEN_WIDTH {
                break;  // Would overflow screen width
            }
            
            self.draw_char(ch, cursor_x, y, color);
            cursor_x += FONT_WIDTH + CHAR_SPACING;
        }
    }
    
    /// Draw multiple lines of text with automatic line spacing
    pub fn draw_text_lines(&mut self, lines: &[&str], start_x: usize, start_y: usize, color: u8) {
        let mut cursor_y = start_y;
        
        for line in lines {
            if cursor_y + FONT_HEIGHT > SCREEN_HEIGHT {
                break;  // Would overflow screen height
            }
            
            self.draw_string(line, start_x, cursor_y, color);
            cursor_y += FONT_HEIGHT + LINE_SPACING;
        }
    }
    
    /// Render the help screen content
    pub fn render_help_screen(&mut self) {
        // Clear screen with dark blue background
        self.clear(1);
        
        let help_text = [
            "SOFT65C02 GRAPHICS DEMO",
            "1: HELP        2: Game of Life",
            "3: Mandelbrot  4: Space Curves",

            "Use following keys in game:",

            "Game of Life:",
            "  R:Random Pattern",
            "Mandelbrot Set:",
            "  Arrows: Pan, +/-: Zoom",
            "  I/D: Iters,  R: Reset",
            "Space Curves: As Mandel +",
            "  S: Spd, C: Color, T: Type",
            "  SPC: Pause, F: Refresh",
        ];
        
        // Draw title in bright white
        self.draw_string(help_text[0], 18, 2, 15);

        self.draw_text_lines(&help_text[1..3], 2, 10, 9); // <keys>
        self.draw_text_lines(&help_text[3..4], 8, 26, 15); // use...
        self.draw_text_lines(&help_text[4..6], 8, 35, 10); // game
        self.draw_text_lines(&help_text[6..9], 8, 50, 13); // mandel
        self.draw_text_lines(&help_text[9..12], 8, 72, 12); // curves
    }
    
    /// Write the frame buffer to video memory
    pub fn write_to_memory(&mut self, memory: &mut Memory) {
        // Clear write buffer
        self.write_buffer.fill(0);
        
        // Pack pixels into nibbles (2 pixels per byte)
        for y in 0..SCREEN_HEIGHT {
            let row_offset = y * BYTES_PER_ROW;
            for x in 0..SCREEN_WIDTH {
                let byte_index = row_offset + (x / 2);
                let is_upper_nibble = (x % 2) == 1;
                let pixel_value = self.frame_buffer[y * SCREEN_WIDTH + x];
                
                if byte_index < self.write_buffer.len() {
                    if is_upper_nibble {
                        self.write_buffer[byte_index] |= pixel_value << 4;
                    } else {
                        self.write_buffer[byte_index] |= pixel_value;
                    }
                }
            }
        }
        
        // Write to video buffer
        memory.write(VIDEO_BUFFER_START, &self.write_buffer).unwrap();
    }
    
    /// Process keyboard input (help screen is mostly static)
    pub fn process_keyboard_input(&mut self, _key_code: u8, _memory: &mut Memory) -> bool {
        // Help screen doesn't process input - mode switching is handled by game.s
        false
    }
    
    /// Compute next generation (no-op for help screen)
    pub fn compute_next_generation(&mut self) {
        // Help screen is static, no generation needed
    }
}

/// Get the palette for the help screen
pub fn get_help_palette() -> Vec<u8> {
    // Create a nice palette for the help screen
    vec![
        // 0: Black
        0x00, 0x00, 0x00,
        // 1: Dark Blue (background)
        0x00, 0x00, 0x40,
        // 2: Dark Green
        0x00, 0x40, 0x00,
        // 3: Dark Cyan
        0x00, 0x40, 0x40,
        // 4: Dark Red
        0x40, 0x00, 0x00,
        // 5: Dark Magenta
        0x40, 0x00, 0x40,
        // 6: Brown
        0x40, 0x40, 0x00,
        // 7: Light Gray
        0xC0, 0xC0, 0xC0,
        // 8: Dark Gray
        0x80, 0x80, 0x80,
        // 9: Blue
        0x00, 0x00, 0xFF,
        // 10: Green
        0x00, 0xFF, 0x00,
        // 11: Cyan (text color)
        0x00, 0xFF, 0xFF,
        // 12: Red
        0xFF, 0x00, 0x00,
        // 13: Magenta
        0xFF, 0x00, 0xFF,
        // 14: Yellow
        0xFF, 0xFF, 0x00,
        // 15: White (title color)
        0xFF, 0xFF, 0xFF,
    ]
}

// 4x6 font data - converted from font.h
// see https://hackaday.io/project/6309-vga-graphics-over-spi-and-serial-vgatonic/log/20759-a-tiny-4x6-pixel-font-that-will-fit-on-almost-any-microcontroller-license-mit
// Each character is represented by 2 bytes encoding 6 rows of 4 pixels
// The font covers ASCII characters from space (32) to DEL (127)
const FONT_4X6: [[u8; 2]; 96] = [
    [0x00, 0x00],   // SPACE
    [0x49, 0x08],   // '!'
    [0xb4, 0x00],   // '"'
    [0xbe, 0xf6],   // '#'
    [0x7b, 0x7a],   // '$'
    [0xa5, 0x94],   // '%'
    [0x55, 0xb8],   // '&'
    [0x48, 0x00],   // '''
    [0x29, 0x44],   // '('
    [0x44, 0x2a],   // ')'
    [0x15, 0xa0],   // '*'
    [0x0b, 0x42],   // '+'
    [0x00, 0x50],   // ','
    [0x03, 0x02],   // '-'
    [0x00, 0x08],   // '.'
    [0x25, 0x90],   // '/'
    [0x76, 0xba],   // '0'
    [0x59, 0x5c],   // '1'
    [0xc5, 0x9e],   // '2'
    [0xc5, 0x38],   // '3'
    [0x92, 0xe6],   // '4'
    [0xf3, 0x3a],   // '5'
    [0x73, 0xba],   // '6'
    [0xe5, 0x90],   // '7'
    [0x77, 0xba],   // '8'
    [0x77, 0x3a],   // '9'
    [0x08, 0x40],   // ':'
    [0x08, 0x50],   // ';'
    [0x2a, 0x44],   // '<'
    [0x1c, 0xe0],   // '='
    [0x88, 0x52],   // '>'
    [0xe5, 0x08],   // '?'
    [0x56, 0x8e],   // '@'
    [0x77, 0xb6],   // 'A'
    [0x77, 0xb8],   // 'B'
    [0x72, 0x8c],   // 'C'
    [0xd6, 0xba],   // 'D'
    [0x73, 0x9e],   // 'E'
    [0x73, 0x92],   // 'F'
    [0x72, 0xae],   // 'G'
    [0xb7, 0xb6],   // 'H'
    [0xe9, 0x5c],   // 'I'
    [0x64, 0xaa],   // 'J'
    [0xb7, 0xb4],   // 'K'
    [0x92, 0x9c],   // 'L'
    [0xbe, 0xb6],   // 'M'
    [0xd6, 0xb6],   // 'N'
    [0x56, 0xaa],   // 'O'
    [0xd7, 0x92],   // 'P'
    [0x76, 0xee],   // 'Q'
    [0x77, 0xb4],   // 'R'
    [0x71, 0x38],   // 'S'
    [0xe9, 0x48],   // 'T'
    [0xb6, 0xae],   // 'U'
    [0xb6, 0xaa],   // 'V'
    [0xb6, 0xf6],   // 'W'
    [0xb5, 0xb4],   // 'X'
    [0xb5, 0x48],   // 'Y'
    [0xe5, 0x9c],   // 'Z'
    [0x69, 0x4c],   // '['
    [0x91, 0x24],   // '\'
    [0x64, 0x2e],   // ']'
    [0x54, 0x00],   // '^'
    [0x00, 0x1c],   // '_'
    [0x44, 0x00],   // '`'
    [0x0e, 0xae],   // 'a'
    [0x9a, 0xba],   // 'b'
    [0x0e, 0x8c],   // 'c'
    [0x2e, 0xae],   // 'd'
    [0x0e, 0xce],   // 'e'
    [0x56, 0xd0],   // 'f'
    [0x55, 0x3b],   // 'g'
    [0x93, 0xb4],   // 'h'
    [0x41, 0x44],   // 'i'
    [0x41, 0x51],   // 'j'
    [0x97, 0xb4],   // 'k'
    [0x49, 0x44],   // 'l'
    [0x17, 0xb6],   // 'm'
    [0x1a, 0xb6],   // 'n'
    [0x0a, 0xaa],   // 'o'
    [0xd6, 0xd3],   // 'p'
    [0x76, 0x67],   // 'q'
    [0x17, 0x90],   // 'r'
    [0x0f, 0x38],   // 's'
    [0x9a, 0x8c],   // 't'
    [0x16, 0xae],   // 'u'
    [0x16, 0xba],   // 'v'
    [0x16, 0xf6],   // 'w'
    [0x15, 0xb4],   // 'x'
    [0xb5, 0x2b],   // 'y'
    [0x1c, 0x5e],   // 'z'
    [0x6b, 0x4c],   // '{'
    [0x49, 0x48],   // '|'
    [0xc9, 0x5a],   // '}'
    [0x54, 0x00],   // '~'
    [0x56, 0xe2],   // DEL
];
