use super::*;

pub fn ldx(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode.
        solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("LDX instruction must have operands, crashing the application");

    registers.register_x = memory.read(target_address, 1).unwrap()[0];
    registers.set_n_flag(registers.register_x & 0b10000000 != 0);
    registers.set_z_flag(registers.register_x == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[Y=0x{:02x}][S={}]", registers.register_x, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_ldx() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xa0, "ldx", AddressingMode::Immediate([0x0a]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa0, 0x0a]);
        registers.register_x = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("ldx".to_owned(), log_line.mnemonic);
        assert_eq!(0x0a, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_ldx_negative() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "LDX", AddressingMode::Immediate([0x8a]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x8a]);
        registers.register_x = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("LDX".to_owned(), log_line.mnemonic);
        assert_eq!(0x8a, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_ldx_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "LDX", AddressingMode::Immediate([0x00]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x00]);
        registers.register_x = 0x10;
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
}


