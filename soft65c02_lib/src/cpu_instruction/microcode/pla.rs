use super::*;

pub fn pla(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.accumulator = registers.stack_pull(memory)?;
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[A=0x{:02x}][SP=0x{:02x}][S={}]",
            registers.accumulator,
            registers.stack_pointer,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_pla() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x68, "PLA", AddressingMode::Implied, pla);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x68, 0x0a]);
        memory.write(STACK_BASE_ADDR + 0x00ff, &[0x10]).unwrap();
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("PLA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, registers.accumulator);
        assert_eq!(0xff, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(4, log_line.cycles); // Implied: 4 cycles
        assert_eq!("#0x1000: (68)          PLA                      [A=0x10][SP=0xff][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_pla_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x68, "PLA", AddressingMode::Implied, pla);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x68, 0x0a]);
        memory.write(STACK_BASE_ADDR + 0x00ff, &[0x00]).unwrap();
        registers.accumulator = 0x10;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(!registers.n_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert_eq!(4, log_line.cycles); // Implied: 4 cycles
        assert_eq!("#0x1000: (68)          PLA                      [A=0x00][SP=0xff][S=nv-BdiZc][4]", log_line.to_string());
    }

    #[test]
    fn test_pla_neg() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x68, "PLA", AddressingMode::Implied, pla);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x68, 0x0a]);
        memory.write(STACK_BASE_ADDR + 0x00ff, &[0x81]).unwrap();
        registers.accumulator = 0x10;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x81, registers.accumulator);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(registers.n_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert_eq!(4, log_line.cycles); // Implied: 4 cycles
        assert_eq!("#0x1000: (68)          PLA                      [A=0x81][SP=0xff][S=Nv-Bdizc][4]", log_line.to_string());
    }
}
