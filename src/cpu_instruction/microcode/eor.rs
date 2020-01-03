use super::*;

pub fn eor(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("No operand given to EOR instruction, crashing application.");

    let byte = memory.read(target_address, 1)?[0];
    registers.accumulator = registers.accumulator ^ byte;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[A=0x{:02x}][S={}]", registers.accumulator, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_eor() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "eor", AddressingMode::Immediate([0x0a]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x02;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("eor".to_owned(), log_line.mnemonic);
        assert_eq!(0x08, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
    }
}
