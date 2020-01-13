extern crate minifb;

pub mod memory;
mod registers;
mod addressing_mode;
mod cpu_instruction;
mod processing_unit;

pub const VERSION:&'static str = "0.1.0";
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

/*
 * Logical execution breakpoint
 * Expressions are evaluated AFTER each instruction is executed.
 * In all cases the execution is stopped if the CP has not changed since last
 * execution. This can occure with a STP instruction of a 0xfe branching or
 * jumping at the same address.
 *
 * LEB might take expressions like:
 * "false" => step by step execution.
 * "CP=0x3456" stops when the command pointer reaches that address.
 * "A>0xa8 & S=0b11000001" stops if the accumulator value is greater than 0xa8
 * and NVZ status flags are set.
 * "#0x1234=0xfa" stops if the specified address matches.
 * "OP=CLC" break if the last executed opcode was CLC.
 */
