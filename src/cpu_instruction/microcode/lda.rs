use super::*;

pub fn lda(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode.
        solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("LDA instruction must have operands, crashing the application");

    registers.accumulator = memory.read(target_address, 1)?[0];
    registers.set_n_flag(registers.accumulator & 0b10000000 != 0);
    registers.set_z_flag(registers.accumulator == 0);
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
    fn test_lda() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "LDA", AddressingMode::ZeroPage([0x0a]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x0a]);
        registers.accumulator = 0x10;
        memory.write(0x000a, vec![0x5a]).unwrap();
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("LDA".to_owned(), log_line.mnemonic);
        assert_eq!(0x5a, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_lda_negative() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "LDA", AddressingMode::ZeroPage([0x0a]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x0a]);
        registers.accumulator = 0x10;
        memory.write(0x000a, vec![0x80]).unwrap();
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("LDA".to_owned(), log_line.mnemonic);
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_lda_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "LDA", AddressingMode::ZeroPage([0x0a]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x0a]);
        registers.accumulator = 0x10;
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
}
