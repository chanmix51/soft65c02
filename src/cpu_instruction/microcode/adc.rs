use super::*;

/*
 * ADC - Add with carry
 *
 * The 65C02 has only one instruction for addition, an addition with carry.
 *
 * TODO: handle the Decimal mode.
 */
pub fn adc(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("ADC must have operands, crashing the application");
    let i:u16 = memory.read(target_address, 1)?[0] as u16
        + registers.accumulator as u16
        + { if registers.c_flag_is_set() { 1 } else { 0 } };

    registers.accumulator = u16::to_le_bytes(i % 0x100)[0];
    registers.set_c_flag(i > 0xff);
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0b10000000 != 0);
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
    fn test_adc() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("ADC".to_owned(), log_line.mnemonic);
        assert_eq!(0x32, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_adc_with_carry() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(true);
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x33, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_adc_set_carry() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0xf8;
        registers.set_c_flag(false);
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_adc_set_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0xf6;
        registers.set_c_flag(false);
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_adc_set_negative() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x7a;
        registers.set_c_flag(false);
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x84, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
