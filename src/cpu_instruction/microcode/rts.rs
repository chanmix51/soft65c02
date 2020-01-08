use super::*;

pub fn rts(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    let cp_lo = registers.stack_pull(memory)?;
    let cp_hi = registers.stack_pull(memory)?;
    registers.command_pointer = ((cp_hi as usize) << 8 | cp_lo as usize) + 1;

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[CP=0x{:04X}]", registers.command_pointer)
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_rts() {
        let cpu_instruction = CPUInstruction::new(0x8000, 0xca, "RTS", AddressingMode::Implied, rts);
        let (mut memory, mut registers) = get_stuff(0x8000, vec![0x00]);
        memory.write(STACK_BASE_ADDR + 0xfe, vec![0x09, 0x10]).unwrap();
        registers.stack_pointer = 0xfd;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("RTS".to_owned(), log_line.mnemonic);
        assert_eq!(0x100a, registers.command_pointer);
        assert_eq!(0xff, registers.stack_pointer);
    }
}


