use super::*;

pub fn tax(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.register_x = registers.accumulator;
    registers.set_n_flag(registers.register_x & 0b10000000 != 0);
    registers.set_z_flag(registers.register_x == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[X=0x{:02x}][S={}]",
            registers.register_x,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_tax() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xAA, "TAX", AddressingMode::Implied, tax);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xAA]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TAX".to_owned(), log_line.mnemonic);
        assert_eq!(0x28, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // TAX takes 2 cycles
        assert_eq!("#0x1000: (aa)          TAX                      [X=0x28][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_tax_with_zero_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xAA, "TAX", AddressingMode::Implied, tax);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xAA]);
        registers.accumulator = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (aa)          TAX                      [X=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_tax_with_negative_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xAA, "TAX", AddressingMode::Implied, tax);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xAA]);
        registers.accumulator = 0xF8;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xF8, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (aa)          TAX                      [X=0xf8][S=Nv-Bdizc][2]", log_line.to_string());
    }
}
