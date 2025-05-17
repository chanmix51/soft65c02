use super::*;

pub fn rts(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let cp_lo = registers.stack_pull(memory)?;
    let cp_hi = registers.stack_pull(memory)?;
    registers.command_pointer = ((cp_hi as usize) << 8 | cp_lo as usize) + 1;

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[CP=0x{:04X}][SP=0x{:02x}][S={}]",
            registers.command_pointer,
            registers.stack_pointer,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_rts() {
        let cpu_instruction =
            CPUInstruction::new(0x8000, 0x60, "RTS", AddressingMode::Implied, rts);
        let (mut memory, mut registers) = get_stuff(0x8000, vec![0x60]);
        memory.write(STACK_BASE_ADDR + 0xfe, &[0x09, 0x10]).unwrap();
        registers.stack_pointer = 0xfd;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("RTS".to_owned(), log_line.mnemonic);
        assert_eq!(0x100a, registers.command_pointer); // Return address + 1
        assert_eq!(0xff, registers.stack_pointer);
        assert_eq!(6, log_line.cycles); // RTS always takes 6 cycles
        assert_eq!("#0x8000: (60)          RTS                      [CP=0x100A][SP=0xff][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_rts_with_jsr() {
        // First do JSR
        let jsr_instruction = CPUInstruction::new(
            0x1000,
            0x20,
            "JSR",
            AddressingMode::Absolute([0x0a, 0x20]),
            jsr,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x20, 0x0a, 0x20]);
        registers.stack_pointer = 0xff;
        let _jsr_log = jsr_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();

        // Then do RTS
        let rts_instruction =
            CPUInstruction::new(0x200a, 0x60, "RTS", AddressingMode::Implied, rts);
        let log_line = rts_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        
        assert_eq!(0x1003, registers.command_pointer); // Original JSR location + 3
        assert_eq!(0xff, registers.stack_pointer); // Stack pointer restored
        assert_eq!(6, log_line.cycles); // RTS always takes 6 cycles
        assert_eq!("#0x200A: (60)          RTS                      [CP=0x1003][SP=0xff][S=nv-Bdizc][6]", log_line.to_string());
    }
}
