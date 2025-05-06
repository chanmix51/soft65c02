use super::*;

pub fn cld(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.set_d_flag(false);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[S={}]", registers.format_status()),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_cld() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xD8, "CLD", AddressingMode::Implied, cld);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xD8]);
        registers.set_d_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("CLD".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(!registers.d_flag_is_set());
        assert_eq!(2, log_line.cycles); // CLD takes 2 cycles
        assert_eq!("#0x1000: (d8)          CLD                      [S=nv-Bdizc][2]", log_line.to_string());
    }
}
