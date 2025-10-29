#[allow(clippy::module_inception)]
mod cpu_instruction;
pub mod microcode;

pub const INIT_VECTOR_ADDR: usize = 0xfffc;
pub const INTERRUPT_VECTOR_ADDR: usize = 0xfffe;

pub use cpu_instruction::{CPUInstruction, LogLine, RegisterState};
