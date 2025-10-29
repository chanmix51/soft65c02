use super::*;

pub fn pha(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.stack_push(memory, registers.accumulator)?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[SP=0x{:02x}]", registers.stack_pointer),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_pha() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x48, "PHA", AddressingMode::Implied, pha);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x48, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("PHA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, memory.read(STACK_BASE_ADDR + 0x00ff, 1).unwrap()[0]);
        assert_eq!(0xfe, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(3, log_line.cycles); // PHA takes 3 cycles
        assert_eq!("#0x1000: (48)          PHA                      [SP=0xfe][3]", log_line.to_string());
    }
}
