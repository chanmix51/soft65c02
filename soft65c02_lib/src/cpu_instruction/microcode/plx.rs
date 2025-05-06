use super::*;

pub fn plx(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.register_x = registers.stack_pull(memory)?;
    registers.set_z_flag(registers.register_x == 0);
    registers.set_n_flag(registers.register_x & 0x80 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[X=0x{:02x}][SP=0x{:02x}][S={}]",
            registers.register_x,
            registers.stack_pointer,
            registers.format_status()
        ),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_plx() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xfa, "PLX", AddressingMode::Implied, plx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xfa, 0x0a]);
        memory.write(STACK_BASE_ADDR + 0x00ff, &[0x10]).unwrap();
        registers.register_x = 0x00;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("PLX".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, registers.register_x);
        assert_eq!(0xff, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(!registers.n_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert_eq!(4, log_line.cycles); // Implied: 4 cycles
        assert_eq!("#0x1000: (fa)          PLX                      [X=0x10][SP=0xff][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_plx_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xfa, "PLX", AddressingMode::Implied, plx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xfa, 0x0a]);
        memory.write(STACK_BASE_ADDR + 0x00ff, &[0x00]).unwrap();
        registers.register_x = 0x10;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(!registers.n_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert_eq!(4, log_line.cycles); // Implied: 4 cycles
        assert_eq!("#0x1000: (fa)          PLX                      [X=0x00][SP=0xff][S=nv-BdiZc][4]", log_line.to_string());
    }

    #[test]
    fn test_plx_neg() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xfa, "PLX", AddressingMode::Implied, plx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xfa, 0x0a]);
        memory.write(STACK_BASE_ADDR + 0x00ff, &[0x81]).unwrap();
        registers.register_x = 0x10;
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x81, registers.register_x);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(registers.n_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert_eq!(4, log_line.cycles); // Implied: 4 cycles
        assert_eq!("#0x1000: (fa)          PLX                      [X=0x81][SP=0xff][S=Nv-Bdizc][4]", log_line.to_string());
    }
}
