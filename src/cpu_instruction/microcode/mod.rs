mod error;
pub use error::{MicrocodeError, Result};

pub use crate::cpu_instruction::{CPUInstruction, LogLine};
pub use crate::registers::Registers;
pub use crate::memory::MemoryStack as Memory;
pub use crate::memory::{AddressableIO, little_endian, MemoryError};
pub use crate::addressing_mode::*;
pub use super::{STACK_BASE_ADDR, INTERRUPT_VECTOR_ADDR, INIT_VECTOR_ADDR};

mod adc;
mod and;
mod asl;
mod bcc;
mod bcs;
mod beq;
mod bit;
mod bmi;
mod bne;
mod bpl;
mod bra;
mod brk;
mod bvc;
mod bvs;
mod clc;
mod cld;
mod cli;
mod clv;
mod cmp;
mod dex;
mod dey;
mod eor;
mod inc;
mod inx;
mod iny;
mod jmp;
mod lda;
mod ldx;
mod ldy;
mod nop;
mod pha;
mod pla;
mod sbc;
mod sta;
mod stp;
mod stx;
mod stz;
mod tax;

pub use adc::adc;
pub use and::and;
pub use asl::asl;
pub use bcc::bcc;
pub use bcs::bcs;
pub use beq::beq;
pub use bit::bit;
pub use bmi::bmi;
pub use bne::bne;
pub use bpl::bpl;
pub use bra::bra;
pub use brk::brk;
pub use bvc::bvc;
pub use bvs::bvs;
pub use clc::clc;
pub use cld::cld;
pub use cli::cli;
pub use clv::clv;
pub use cmp::cmp;
pub use dex::dex;
pub use dey::dey;
pub use eor::eor;
pub use inc::inc;
pub use inx::inx;
pub use iny::iny;
pub use jmp::jmp;
pub use lda::lda;
pub use ldx::ldx;
pub use ldy::ldy;
pub use nop::nop;
pub use pha::pha;
pub use pla::pla;
pub use sbc::sbc;
pub use sta::sta;
pub use stp::stp;
pub use stx::stx;
pub use stz::stz;
pub use tax::tax;

fn stack_push(memory: &mut Memory, registers: &mut Registers, byte: u8) -> std::result::Result<(), MemoryError> {
    memory.write(STACK_BASE_ADDR + registers.stack_pointer as usize, vec![byte])?;
    let (sp, _) = registers.stack_pointer.overflowing_sub(1);
    registers.stack_pointer = sp;

    Ok(())
}

fn stack_pull(memory: &mut Memory, registers: &mut Registers) -> std::result::Result<u8, MemoryError> {
    let (sp, _) = registers.stack_pointer.overflowing_add(1);
    registers.stack_pointer = sp;
    Ok(memory.read(STACK_BASE_ADDR + registers.stack_pointer as usize, 1)?[0])
}
