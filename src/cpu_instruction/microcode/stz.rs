use super::*;

pub fn stz(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution
        .target_address
        .expect("STZ must have operands, crashing the application");

    memory.write(target_address, vec![0x00])?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(&cpu_instruction, resolution, String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_stz() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0xca,
            "STZ",
            AddressingMode::Absolute([0x00, 0x10]),
            stz,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x00, 0x10]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("STZ".to_owned(), log_line.mnemonic);
        assert_eq!(0x00, memory.read(0x1000, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
    }
}
