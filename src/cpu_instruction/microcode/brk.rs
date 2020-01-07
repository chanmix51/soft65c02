use super::*;

pub fn brk(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    registers.set_b_flag(true);
    let bytes = usize::to_le_bytes(registers.command_pointer);
    registers.stack_push(memory, bytes[0])?;
    registers.stack_push(memory, bytes[1])?;
    registers.stack_push(memory, registers.status_register)?;
    registers.command_pointer = little_endian(memory.read(INTERRUPT_VECTOR_ADDR, 2)?);

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[CP=0x{:04X}][SP=0x{:02x}]", registers.command_pointer, registers.stack_pointer)
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_brk() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "BRK", AddressingMode::Implied, brk);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x00]);
        memory.write(0xfffe, vec![0x00, 0xf0]);
        registers.stack_pointer = 0xff;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("BRK".to_owned(), log_line.mnemonic);
        assert_eq!(0xf000, registers.command_pointer);
        assert_eq!(0xfc, registers.stack_pointer);
        assert!(registers.b_flag_is_set());
    }
}
