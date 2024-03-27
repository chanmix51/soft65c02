use super::*;

pub fn ora(
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
        .expect("ORA must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];
    registers.accumulator |= byte;
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        &cpu_instruction,
        resolution,
        format!(
            "[A=0x{:02x}][S={}]",
            registers.accumulator,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_ora() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ORA", AddressingMode::Immediate([0x0a]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x00, 0x0a, 0x02]);
        registers.accumulator = 0x22;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("ORA".to_owned(), log_line.mnemonic);
        assert_eq!(0x2A, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_ora_set_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ORA", AddressingMode::Immediate([0x00]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x00, 0x02]);
        registers.accumulator = 0x00;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_ora_set_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ORA", AddressingMode::Immediate([0x00]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x00]);
        registers.accumulator = 0xd5;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xd5, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
