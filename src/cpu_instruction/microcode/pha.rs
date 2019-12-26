use crate::cpu_instruction::{CPUInstruction, LogLine, STACK_BASE_ADDR};
use crate::registers::Registers;
use crate::memory::RAM as Memory;
use crate::addressing_mode::*;
use super::{MicrocodeError, Result};

pub fn pha(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode.solve(registers.command_pointer, memory, registers)?;

    memory.write(STACK_BASE_ADDR + (registers.stack_pointer as usize), vec![registers.accumulator])?;
    registers.stack_pointer -= 1;

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine {
        address:    cpu_instruction.address,
        opcode:     cpu_instruction.opcode,
        mnemonic:   cpu_instruction.mnemonic.clone(),
        resolution: resolution,
        is_simulated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_pha() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "PHA", AddressingMode::Implied, pha);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x48, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("PHA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, memory.read(STACK_BASE_ADDR + 0x00ff, 1).unwrap()[0]);
        assert_eq!(0xfe, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
    }
}

