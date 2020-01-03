use super::*;

pub fn jmp(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("JMP must have an operand, crashing the application");

    registers.command_pointer = target_address;

    Ok(LogLine {
        address:    cpu_instruction.address,
        opcode:     cpu_instruction.opcode,
        mnemonic:   cpu_instruction.mnemonic.clone(),
        resolution: resolution,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_jmp() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "JMP", AddressingMode::Absolute([0x0a, 0x02]), jmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("JMP".to_owned(), log_line.mnemonic);
        assert_eq!(0x020a, registers.command_pointer);
    }
}
