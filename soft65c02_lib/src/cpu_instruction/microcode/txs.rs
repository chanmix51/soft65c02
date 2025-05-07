use super::*;

pub fn txs(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.stack_pointer = registers.register_x;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[SP=0x{:02x}][S={}]",
            registers.stack_pointer,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_txs() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9A, "TXS", AddressingMode::Implied, txs);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9A]);
        registers.register_x = 0x83;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TXS".to_owned(), log_line.mnemonic);
        assert_eq!(0x83, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // TXS takes 2 cycles
        assert_eq!("#0x1000: (9a)          TXS                      [SP=0x83][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_txs_with_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9A, "TXS", AddressingMode::Implied, txs);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9A]);
        registers.register_x = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (9a)          TXS                      [SP=0x00][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_txs_with_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9A, "TXS", AddressingMode::Implied, txs);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9A]);
        registers.register_x = 0xFF;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xFF, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (9a)          TXS                      [SP=0xff][S=nv-Bdizc][2]", log_line.to_string());
    }
}
