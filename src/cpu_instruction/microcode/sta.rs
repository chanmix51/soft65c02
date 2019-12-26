use crate::cpu_instruction::{CPUInstruction, LogLine};
use crate::registers::Registers;
use crate::memory::RAM as Memory;
use crate::addressing_mode::*;
use super::{MicrocodeError, Result};

pub fn sta(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode.solve(registers.command_pointer, memory, registers)?;
    let target_address = match resolution.target_address {
        Some(v) => v,
        None => panic!("Ooops no target address from the addressing mode resolver."),
    };

    memory.write(target_address, vec![registers.accumulator]).unwrap();
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
    fn test_sta() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "STA", AddressingMode::ZeroPage, sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("STA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, memory.read(0x0a, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
    }
}
