mod cpu_instruction;
pub mod microcode;

pub const INIT_VECTOR_ADDR: usize = 0xfffc;
pub const INTERRUPT_VECTOR_ADDR: usize = 0xfffe;

pub use self::cpu_instruction::{CPUInstruction, LogLine};
