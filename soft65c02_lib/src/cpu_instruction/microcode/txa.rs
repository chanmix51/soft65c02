use super::*;

pub fn txa(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.accumulator = registers.register_x;
    registers.set_n_flag(registers.accumulator & 0b10000000 != 0);
    registers.set_z_flag(registers.accumulator == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
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
    fn test_txa() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "TXA", AddressingMode::Implied, txa);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8a]);
        registers.register_x = 0x43;
        registers.accumulator = 0x0a;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TXA".to_owned(), log_line.mnemonic);
        assert_eq!(0x43, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_txa_with_n_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "TXA", AddressingMode::Implied, txa);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8a, 0x0a, 0x02]);
        registers.set_n_flag(false);
        registers.register_x = 0x80;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_txa_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "TXA", AddressingMode::Implied, txa);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8a, 0x0a, 0x02]);
        registers.set_z_flag(false);
        registers.register_x = 0x00;
        registers.accumulator = 0x0a;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }
}
