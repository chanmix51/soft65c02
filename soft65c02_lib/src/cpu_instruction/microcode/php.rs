use super::*;

pub fn php(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.stack_push(memory, registers.get_status_register())?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[SP=0x{:02x}]", registers.stack_pointer),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_php() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x08, "PHP", AddressingMode::Implied, php);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x48, 0x0a]);
        registers.set_d_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("PHP".to_owned(), log_line.mnemonic);
        assert_eq!(
            0b00111000,
            memory.read(STACK_BASE_ADDR + 0x00ff, 1).unwrap()[0]
        );
        assert_eq!(0xfe, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(3, log_line.cycles); // PHP takes 3 cycles
        assert_eq!("#0x1000: (08)          PHP                      [SP=0xfe][3]", log_line.to_string());
    }
}
