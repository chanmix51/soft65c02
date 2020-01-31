extern crate minifb;

mod addressing_mode;
mod cpu_instruction;
pub mod memory;
mod processing_unit;
mod registers;

pub const VERSION: &'static str = "0.1.0";
const INIT_VECTOR: usize = 0xFFFC;
const INTERRUPT_VECTOR: usize = 0xFFFE;

pub use cpu_instruction::{CPUInstruction, LogLine, MicrocodeError};
pub use memory::AddressableIO;
pub use memory::MemoryStack as Memory;
pub use processing_unit::*;
pub use registers::Registers;

pub fn mem_dump(start: usize, len: usize, memory: &Memory) -> Vec<String> {
    let mut output:Vec<String> = vec![];
    if len == 0 { return output }
    let address = start - (start % 16);
    let bytes = memory.read(address, 16 * len).unwrap();

    for lineno in 0..len {
        let mut line = format!("#{:04X}: ", address + lineno * 16);
        for col in 0..15 {
            if col == 7 {
                line.push(' ');
            }
            line = format!("{} {:02x}", line, bytes[16 * lineno + col]);
        }
        output.push(line);
    }

    output
}

pub fn execute(
    memory: &mut Memory,
    registers: &mut Registers,
) -> Result<Vec<LogLine>, MicrocodeError> {
    let mut logs: Vec<LogLine> = vec![];

    loop {
        let cp = registers.command_pointer;
        match processing_unit::execute_step(registers, memory) {
            Ok(v) => logs.push(v),
            Err(v) => break Err(v),
        }

        if registers.command_pointer == cp {
            break Ok(logs);
        }
    }
}
