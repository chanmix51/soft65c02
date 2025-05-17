use super::*;

pub fn sei(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.set_i_flag(true);
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
    fn test_sei() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x78, "SEI", AddressingMode::Implied, sei);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x78]);
        registers.set_i_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("SEI".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(registers.i_flag_is_set());
        assert_eq!(2, log_line.cycles); // SEI takes 2 cycles
        assert_eq!("#0x1000: (78)          SEI                      [S=nv-BdIzc][2]", log_line.to_string());
    }
}
