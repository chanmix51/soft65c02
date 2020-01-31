extern crate minifb;

mod addressing_mode;
mod cpu_instruction;
pub mod memory;
mod processing_unit;
mod registers;

pub const VERSION: &'static str = "0.1.0";
pub const INIT_VECTOR: usize = 0xFFFC;
pub const INTERRUPT_VECTOR: usize = 0xFFFE;

pub use cpu_instruction::{CPUInstruction, LogLine, MicrocodeError};
pub use memory::AddressableIO;
pub use memory::MemoryStack as Memory;
pub use processing_unit::*;
pub use registers::Registers;

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
