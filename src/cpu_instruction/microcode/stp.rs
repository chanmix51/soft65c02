use super::*;

pub fn stp(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    Ok(LogLine::new(&cpu_instruction, resolution, String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_stp() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xdb, "STP", AddressingMode::Implied, stp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xdb]);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("STP".to_owned(), log_line.mnemonic);
        assert_eq!(0x1000, registers.command_pointer);
    }
}
