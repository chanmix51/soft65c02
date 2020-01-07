pub const INIT_VECTOR_ADDR:usize = 0xfffc;
pub const INTERRUPT_VECTOR_ADDR:usize = 0xfffe;

pub mod microcode;
mod cpu_instruction;

pub use cpu_instruction::{CPUInstruction, LogLine};
pub use microcode::MicrocodeError;

