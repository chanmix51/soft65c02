use std::io::prelude::*;
use std::fs::File;

use soft65c02_graphics::PixelsDisplay;
use soft65c02_lib::{AddressableIO, Memory, Registers, execute_step};

fn main() {
    let init_vector: usize = 0x1B00;
    let mut memory = Memory::new_with_ram();
    memory.add_subsystem("VIDEO TERMINAL", 0x0200, PixelsDisplay::new());
    
    // Load palette (create default if file doesn't exist)
    let palette: Vec<u8> = match File::open("../palette.bin") {
        Ok(mut file) => {
            let mut palette = Vec::new();
            file.read_to_end(&mut palette).unwrap();
            palette
        }
        Err(_) => {
            // Create a basic 16-color palette (48 bytes: 16 colors × 3 RGB bytes)
            vec![
                // Black, Dark Blue, Dark Green, Dark Cyan, Dark Red, Dark Magenta, Brown, Light Gray
                0x00, 0x00, 0x00,  0x00, 0x00, 0x80,  0x00, 0x80, 0x00,  0x00, 0x80, 0x80,
                0x80, 0x00, 0x00,  0x80, 0x00, 0x80,  0x80, 0x80, 0x00,  0xC0, 0xC0, 0xC0,
                // Dark Gray, Blue, Green, Cyan, Red, Magenta, Yellow, White
                0x80, 0x80, 0x80,  0x00, 0x00, 0xFF,  0x00, 0xFF, 0x00,  0x00, 0xFF, 0xFF,
                0xFF, 0x00, 0x00,  0xFF, 0x00, 0xFF,  0xFF, 0xFF, 0x00,  0xFF, 0xFF, 0xFF,
            ]
        }
    };
    memory.write(0x0200, &palette).unwrap();
    
    memory.write(init_vector, &dump_program()).unwrap();
    let mut registers = Registers::new(init_vector);
    let mut cp = 0x0000;

    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        println!(
            "{}",
            execute_step(&mut registers, &mut memory).unwrap()
        );
    }
}

fn dump_program() -> Vec<u8> {
    /*
       lda #$0f
       sta $8000
       lda #$00
       tax
    loop:
       ina
       sbc $8000
       adc $0330
       sta $0300,X
       sta $0400,X
       sta $0500,X
       inx
       bne loop
       jmp $1B02
       */
    vec![
        0xa9, 0x0f, 0x8d, 0x00, 0x80, 0xa9, 0x00, 0xaa, 0x1a, 0xed, 0x00, 0x80, 0x6d, 0x30, 0x03,
        0x9d, 0x00, 0x03, 0x9d, 0x00, 0x04, 0x9d, 0x00, 0x05, 0xe8, 0xd0, 0xed, 0x4c, 0x02, 0x1b 
    ]
} 