extern crate minifb;

pub mod memory;
mod registers;
mod addressing_mode;
mod cpu_instruction;
mod processing_unit;

const INIT_VECTOR:usize = 0xFFFC;
const INTERRUPT_VECTOR:usize = 0xFFFE;

pub use memory::MemoryStack as Memory;
pub use memory::AddressableIO;
pub use registers::Registers;
pub use processing_unit::*;
pub use cpu_instruction::{LogLine, CPUInstruction, MicrocodeError};

fn mem_dump(start: usize, end: usize, memory: &Memory) {
    let mut line = String::new();
    let address = start;
    let bytes = memory.read(start, end - start + 1).unwrap();

    while address < end {
        if address % 16 == start % 16 {
            println!("{}", line);
            line = format!("#{:04X}: ", address);
        } else if address % 8 == start % 8 {
            line = format!("{} ", line);
        }

        line = format!("{} {:02x}", line, bytes[address]);
    }

    println!("{}", line);
}

pub fn execute(memory: &mut Memory, registers: &mut Registers) -> Result<Vec<LogLine>, MicrocodeError> {
    let mut logs:Vec<LogLine> = vec![];

    loop {
        let cp = registers.command_pointer;
        match processing_unit::execute_step(registers, memory) {
            Ok(v)  => logs.push(v),
            Err(v) => break Err(v),
        }

        if registers.command_pointer == cp {
            break Ok(logs);
        }
    }
}

