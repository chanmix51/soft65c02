pub const STACK_BASE_ADDR:usize = 0x0100;

pub mod microcode;
mod cpu_instruction;

pub use cpu_instruction::{CPUInstruction, LogLine};
pub use microcode::MicrocodeError;

