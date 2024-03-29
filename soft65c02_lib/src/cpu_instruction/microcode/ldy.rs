use super::*;

pub fn ldy(
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
        .expect("LDY instruction must have operands, crashing the application");

    registers.register_y = memory.read(target_address, 1)?[0];
    registers.set_n_flag(registers.register_y & 0b10000000 != 0);
    registers.set_z_flag(registers.register_y == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[Y=0x{:02x}][S={}]",
            registers.register_y,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_ldy() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xa0, "LDY", AddressingMode::Immediate([0x0a]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa0, 0x0a]);
        registers.register_y = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("LDY".to_owned(), log_line.mnemonic);
        assert_eq!(0x0a, registers.register_y);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_ldy_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "LDY", AddressingMode::Immediate([0x8a]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x8a]);
        registers.register_y = 0x10;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x8a, registers.register_y);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_ldy_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "LDY", AddressingMode::Immediate([0x00]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x00]);
        registers.register_y = 0x10;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_y);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
}
