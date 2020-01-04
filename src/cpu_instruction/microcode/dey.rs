use super::*;

pub fn dey(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    if registers.register_y != 0 {
        registers.register_y -= 1;
    } else {
        registers.register_y = 0xff;
    }
    registers.set_z_flag(registers.register_y == 0);
    registers.set_n_flag(registers.register_y & 0b10000000 != 0);

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[Y=0x{:02x}][S={}]", registers.register_y, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_dex() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEY", AddressingMode::Implied, dey);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.register_y = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("DEY".to_owned(), log_line.mnemonic);
        assert_eq!(0x0f, registers.register_y);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_dex_when_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEY", AddressingMode::Implied, dey);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0xff, registers.register_y);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_dex_when_one() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEY", AddressingMode::Implied, dey);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.register_y = 0x01;
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.register_y);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
}
