use super::*;

pub fn dex(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let (res, _) = registers.register_x.overflowing_sub(1);

    registers.set_z_flag(res == 0);
    registers.set_n_flag(res & 0b10000000 != 0);
    registers.register_x = res;

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[X=0x{:02x}][S={}]", registers.register_x, registers.format_status()),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_dex() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xCA, "DEX", AddressingMode::Implied, dex);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xCA]);
        registers.register_x = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("DEX".to_owned(), log_line.mnemonic);
        assert_eq!(0x0F, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (ca)          DEX                      [X=0x0f][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_dex_with_zero_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xCA, "DEX", AddressingMode::Implied, dex);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xCA]);
        registers.register_x = 0x01;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (ca)          DEX                      [X=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_dex_with_negative_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xCA, "DEX", AddressingMode::Implied, dex);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xCA]);
        registers.register_x = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xFF, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (ca)          DEX                      [X=0xff][S=Nv-Bdizc][2]", log_line.to_string());
    }
}
