use std::io::prelude::*;
use std::fs::File;

use soft65c02::memory::MiniFBMemory;
use soft65c02::{AddressableIO, Memory, Registers};

fn main() {
    use std::fs;
    use std::io::prelude::*;
    use std::{thread, time};

    let init_vector: usize = 0x1B00;
    let mut memory = Memory::new_with_ram();
    assert!(true);
    memory.add_subsystem("VIDEO TERMINAL", 0x0200, MiniFBMemory::new(None));
    {
        let mut file = File::open("palette.bin").unwrap();
        let mut palette:Vec<u8> = Vec::new();
        file.read(&mut palette).unwrap();
        memory.write(0x0200, &palette).unwrap();
    }
    memory.write(init_vector, &dump_program()).unwrap();
    let mut registers = Registers::new(init_vector);
    let mut cp = 0x0000;

    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        println!(
            "{}",
            soft65c02::execute_step(&mut registers, &mut memory).unwrap()
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
