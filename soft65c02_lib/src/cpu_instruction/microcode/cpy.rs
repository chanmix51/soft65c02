use super::*;

pub fn cpy(
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
        .expect("CPY must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];

    registers.set_c_flag(registers.register_y >= byte);
    registers.set_z_flag(registers.register_y == byte);
    registers.set_n_flag(registers.register_y < byte);
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
    fn test_cpy_be() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "CPY", AddressingMode::Immediate([0x0a]), cpy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.register_y = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("CPY".to_owned(), log_line.mnemonic);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_cpy_equ() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "CPY", AddressingMode::Immediate([0x0a]), cpy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.register_y = 0x0a;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_cpy_lt() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "CPY", AddressingMode::Immediate([0x0a]), cpy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.register_y = 0x01;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
