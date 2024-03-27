use super::*;

pub fn cmp(
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
        .expect("cmp must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];

    registers.set_c_flag(registers.accumulator >= byte);
    registers.set_z_flag(registers.accumulator == byte);
    registers.set_n_flag(registers.accumulator < byte);
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
    fn test_cmp_be() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "CMP", AddressingMode::Immediate([0x0a]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("CMP".to_owned(), log_line.mnemonic);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_cmp_equ() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "CMP", AddressingMode::Immediate([0x0a]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x0a;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_cmp_lt() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "CMP", AddressingMode::Immediate([0x0a]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x01;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
