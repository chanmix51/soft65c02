use soft65c02::memory::MiniFBMemory;
use soft65c02::{AddressableIO, LogLine, Memory, Registers};

fn main() {
    use std::fs;
    use std::io::prelude::*;
    use std::{thread, time};

    let init_vector: usize = 0x1B00;
    let mut memory = Memory::new_with_ram();
    assert!(true);
    memory.add_subsystem("VIDEO TERMINAL", 0x0200, MiniFBMemory::new(None));
    memory.write(init_vector, &dump_program()).unwrap();
    let mut registers = Registers::new(init_vector);
    let mut cp = 0x0000;
    let mut f = fs::File::create("log.txt").unwrap();

    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        writeln!(
            f,
            "{}",
            soft65c02::execute_step(&mut registers, &mut memory).unwrap()
        );
        memory.refresh();
        thread::sleep(time::Duration::from_millis(1));
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
       brk
       */
    vec![
        0xa9, 0x0f, 0x8d, 0x00, 0x80, 0xa9, 0x00, 0xaa, 0x1a, 0xed, 0x00, 0x80, 0x6d, 0x30, 0x03,
        0x9d, 0x00, 0x03, 0x9d, 0x00, 0x04, 0x9d, 0x00, 0x05, 0xe8, 0xd0, 0xed, 0x00,
    ]
}
