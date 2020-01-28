use super::*;

/*
 * SBC
 * Seems not to work correctly according to the A65 functional test.
 * The decimal mode is not implemented yet.
 *
 * @see https://github.com/Klaus2m5/6502_65C02_functional_tests
 */
pub fn sbc(
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
        .expect("SBC must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];
    let a = registers.accumulator;
    if registers.d_flag_is_set() {
        let carry = if registers.c_flag_is_set() { 0 } else { 1 };
        let low1 = a & 0x0F;
        let low2 = byte & 0x0F;
        let (sublow, carry ) = if low1 >= (low2 + carry) {
            (low1 - (low2 + carry), 0)
        } else {
            (10 + low1 - (low2 + carry), 1)
        };
        let hi1 = a >> 4;
        let hi2 = byte >> 4;
        let (subhi, carry) = if hi1 >= (hi2 + carry) {
            (hi1 - (hi2 + carry), true)
        } else {
            (10 + hi1 - (hi2 + carry), false)
        };
        registers.accumulator = subhi << 4 | sublow;
        registers.set_c_flag(carry);
    } else {
        let (sub, c) = if !registers.c_flag_is_set() {
            byte.overflowing_add(1)
        } else {
            (byte, false)
        };

        let (res, carry) = a.overflowing_sub(sub);
        registers.accumulator = res;
        registers.set_c_flag(!(carry | c));
    }
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);
    registers.set_v_flag((a ^ registers.accumulator) & !(byte ^ registers.accumulator) & 0x80 != 0);

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        &cpu_instruction,
        resolution,
        format!(
            "(0x{:02x})[A=0x{:02x}][S={}]",
            byte,
            registers.accumulator,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sbc() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x0a]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
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
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x00]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x00, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x28, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_with_no_preceding_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x01]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x01, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x26, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x01]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x01, 0x02]);
        registers.accumulator = 0x01;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0xff]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xff, 0x02]);
        registers.accumulator = 0xfb;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xfc, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_overflow() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x02]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x02, 0x02]);
        registers.accumulator = 0x81;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x7f, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_zero_without_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0xff]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xff, 0x02]);
        registers.accumulator = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(!registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_zero_with_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0xff]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xff, 0x02]);
        registers.accumulator = 0xff;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_sbc_decmode() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x13]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x13, 0x02]);
        registers.accumulator = 0x40;
        registers.set_d_flag(true);
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x27, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
    #[test]
    fn test_sbc_decmode_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "SBC", AddressingMode::Immediate([0x21]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x21, 0x02]);
        registers.accumulator = 0x12;
        registers.set_d_flag(true);
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x91, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
