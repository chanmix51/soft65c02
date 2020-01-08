use super::*;

pub fn sbc(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("SBC must have operands, crashing the application");

    let mut byte = memory.read(target_address, 1)?[0];
    if ! registers.c_flag_is_set() {
        let (r, _) = byte.overflowing_add(1);
        byte = r;
    }
    let a = registers.accumulator;
    let (res, carry) = a.overflowing_sub(byte);
    registers.accumulator = res;
    registers.set_c_flag(!carry);
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);
    registers.set_v_flag((a ^ registers.accumulator) & (byte ^ registers.accumulator) & 0x80 != 0);

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[A=0x{:02x}][S={}]", registers.accumulator, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sbc() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x0a]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("SBC".to_owned(), log_line.mnemonic);
        assert_eq!(0x1e, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_with_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x00]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x00, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x28, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_with_no_preceding_carry() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x01]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x01, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(false);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x26, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x01]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x01, 0x02]);
        registers.accumulator = 0x01;
        registers.set_c_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_negative() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0xff]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xff, 0x02]);
        registers.accumulator = 0xfb;
        registers.set_c_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0xfc, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_overflow() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x02]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x02, 0x02]);
        registers.accumulator = 0x81;
        registers.set_c_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x7f, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
}
