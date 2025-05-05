use super::*;

pub fn sed(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.set_d_flag(true);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[S={}]", registers.format_status()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sed() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xF8, "SED", AddressingMode::Implied, sed);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xF8]);
        registers.set_d_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("SED".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(registers.d_flag_is_set());
        assert_eq!(2, log_line.cycles); // SED takes 2 cycles
        assert_eq!("#0x1000: (f8)          SED                      [S=nv-BDizc][2]", log_line.to_string());
    }
}
