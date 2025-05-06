use super::*;

pub fn clv(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.set_v_flag(false);
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
    fn test_clv() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB8, "CLV", AddressingMode::Implied, clv);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB8]);
        registers.set_v_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("CLV".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // CLV takes 2 cycles
        assert_eq!("#0x1000: (b8)          CLV                      [S=nv-Bdizc][2]", log_line.to_string());
    }
}
