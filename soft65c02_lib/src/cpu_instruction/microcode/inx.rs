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
        &cpu_instruction,
        resolution,
        format!(
            "[X=0x{:02x}][S={}]",
            registers.register_x,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_inx() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "INX", AddressingMode::Implied, inx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.register_x = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("INX".to_owned(), log_line.mnemonic);
        assert_eq!(0x29, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_inx_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "INX", AddressingMode::Implied, inx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.register_x = 0xff;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_inx_with_n_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "INX", AddressingMode::Implied, inx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.register_x = 0xf7;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xf8, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }
}
