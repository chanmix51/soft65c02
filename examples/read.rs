use soft65c02::{Memory, Registers, AddressableIO, LogLine};
use std::io::prelude::*;
use std::fs::File;
use std::io;

fn read_file(filename: &str) -> Vec<u8> {
    let mut f = File::open(filename).unwrap();
    let mut buffer:Vec<u8> = vec![];
    f.read_to_end(&mut buffer).unwrap();
    buffer
}

fn main(){
    let init_vector:usize = 0x400;
    let mut memory = Memory::new_with_ram();
    {
        let buffer = read_file("6502_functional_test.bin");
        memory.write(0x00, buffer).unwrap();
    }
    let mut registers = Registers::new(init_vector);
    let mut cp:usize = 0x0000;
    let mut buffer = File::create("6502_functional_test.log").unwrap();
    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        let logline = soft65c02::execute_step(&mut registers, &mut memory).unwrap();
        let line = format!("{}\n", logline);
        buffer.write(line.as_bytes());
    }
}


