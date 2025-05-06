use super::*;

pub fn inx(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    if registers.register_x == 255 {
        registers.register_x = 0;
        registers.set_z_flag(true);
    } else {
        registers.register_x += 1;
        registers.set_z_flag(false);
    }
    registers.set_n_flag(registers.register_x & 0b10000000 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[X=0x{:02x}][S={}]",
            registers.register_x,
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
    fn test_inx() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xE8, "INX", AddressingMode::Implied, inx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xE8]);
        registers.register_x = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("INX".to_owned(), log_line.mnemonic);
        assert_eq!(0x29, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // INX takes 2 cycles
        assert_eq!("#0x1000: (e8)          INX                      [X=0x29][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_inx_with_zero_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xE8, "INX", AddressingMode::Implied, inx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xE8]);
        registers.register_x = 0xFF;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (e8)          INX                      [X=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_inx_with_negative_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xE8, "INX", AddressingMode::Implied, inx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xE8]);
        registers.register_x = 0x7F;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (e8)          INX                      [X=0x80][S=Nv-Bdizc][2]", log_line.to_string());
    }
}
