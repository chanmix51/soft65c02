use super::*;

pub fn pla(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    if registers.stack_pointer == 0xff {
        registers.stack_pointer = 0x00;
    } else {
        registers.stack_pointer += 1;
    }

    registers.accumulator = registers.stack_pull(memory)?;
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!(
                "[A=0x{:02x}][SP=0x{:02x}][S={}]",
                registers.accumulator,
                registers.stack_pointer,
                registers.format_status()
                )
            )
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_pla() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0x08, "PLA", AddressingMode::Implied, pla);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x08, 0x0a]);
        memory.write(0x01ff, vec![0x10]);
        registers.accumulator = 0x00;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("PLA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, registers.accumulator);
        assert_eq!(0xff, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(!registers.n_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert_eq!(format!("#0x1000: (08)          PLA                      [A=0x10][SP=0xff][S=nv-Bdizc]"), format!("{}", log_line));
    }

    #[test]
    fn test_pla_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "PLA", AddressingMode::Implied, pla);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x48, 0x0a]);
        memory.write(0x01ff, vec![0x00]);
        registers.accumulator = 0x10;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(!registers.n_flag_is_set());
        assert!(registers.z_flag_is_set());
    }

    #[test]
    fn test_pla_neg() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "PLA", AddressingMode::Implied, pla);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x48, 0x0a]);
        memory.write(0x01ff, vec![0x81]);
        registers.accumulator = 0x10;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x81, registers.accumulator);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(registers.n_flag_is_set());
        assert!(!registers.z_flag_is_set());
    }
}


