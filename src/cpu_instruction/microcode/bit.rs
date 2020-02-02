use super::*;

pub fn bit(
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
        .expect("BIT must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];
    registers.set_z_flag(registers.accumulator & byte == 0);

    /*
     * see http://forum.6502.org/viewtopic.php?f=2&t=2241&p=27243#p27239
     */
    match cpu_instruction.addressing_mode {
        AddressingMode::Immediate(_) => {},
        _   => {
            registers.set_v_flag(byte & 0b01000000 != 0);
            registers.set_n_flag(byte & 0b10000000 != 0);
        },
    };
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        &cpu_instruction,
        resolution,
        format!("[S={}]", registers.format_status()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bit() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "BIT", AddressingMode::Immediate([0x0a]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a, 0x02]);
        registers.accumulator = 0x03;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BIT".to_owned(), log_line.mnemonic);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_bit_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "bit", AddressingMode::ZeroPage([0xa0]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0xa0, 0x02]);
        registers.accumulator = 0x03;
        memory.write(0xa0, &vec![0xba]).unwrap();
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
    }

    #[test]
    fn test_bit_negative_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "bit", AddressingMode::Immediate([0xba]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0xba, 0x02]);
        registers.accumulator = 0x03;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
    }

    #[test]
    fn test_bit_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "bit", AddressingMode::Immediate([0x03]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x03, 0x02]);
        registers.accumulator = 0x04;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
    }

    #[test]
    fn test_bit_overflow() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "bit", AddressingMode::ZeroPage([0xa0]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0xa0, 0x02]);
        registers.accumulator = 0x03;
        memory.write(0xa0, &vec![0x4d]).unwrap();
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(registers.v_flag_is_set());
    }

    #[test]
    fn test_bit_overflow_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "bit", AddressingMode::Immediate([0x4d]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x4d, 0x02]);
        registers.accumulator = 0x03;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
    }
}
