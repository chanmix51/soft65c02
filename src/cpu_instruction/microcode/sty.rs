use super::*;

pub fn sty(
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
        .expect("STY must have operands, crashing the application");

    memory.write(target_address, &vec![registers.register_y])?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(&cpu_instruction, resolution, String::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sty() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "STY", AddressingMode::ZeroPage([0x0a]), sty);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.register_y = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("STY".to_owned(), log_line.mnemonic);
        assert_eq!(0x28, memory.read(0x0a, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
    }
}
