use super::*;

pub fn tya(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.accumulator = registers.register_y;
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
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_tya() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x98, "TYA", AddressingMode::Implied, tya);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x98]);
        registers.register_y = 0x43;
        registers.accumulator = 0x0a;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TYA".to_owned(), log_line.mnemonic);
        assert_eq!(0x43, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (98)          TYA                      [A=0x43][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_tya_with_n_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x98, "TYA", AddressingMode::Implied, tya);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x98]);
        registers.set_n_flag(false);
        registers.register_y = 0x80;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (98)          TYA                      [A=0x80][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_tya_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x98, "TYA", AddressingMode::Implied, tya);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x98]);
        registers.set_z_flag(false);
        registers.register_y = 0x00;
        registers.accumulator = 0x0a;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (98)          TYA                      [A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }
}
