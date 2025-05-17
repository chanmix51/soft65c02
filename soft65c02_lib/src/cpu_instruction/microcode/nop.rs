use super::*;

pub fn nop(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[S={}]", registers.format_status())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_nop_implied() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xea, "NOP", AddressingMode::Implied, nop);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xea]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("NOP".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Implied: 2 cycles
        assert_eq!("#0x1000: (ea)          NOP                      [S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_nop_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x02, "NOP", AddressingMode::Immediate([0x42]), nop);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x02, 0x42]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("NOP".to_owned(), log_line.mnemonic);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (02 42)       NOP  #$42     (#0x1001)  [S=nv-Bdizc][2]", log_line.to_string());
    }
}
