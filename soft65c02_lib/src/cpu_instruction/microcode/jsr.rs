use super::*;

pub fn jsr(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution
        .target_address
        .expect("JSR must have an operand, crashing the application");

    let bytes = usize::to_le_bytes(registers.command_pointer + resolution.operands.len());
    registers.stack_push(memory, bytes[1])?;
    registers.stack_push(memory, bytes[0])?;
    registers.command_pointer = target_address;

    Ok(LogLine::new(
        &cpu_instruction,
        resolution,
        format!(
            "[CP=0x{:04x}][SP=0x{:02x}]",
            registers.command_pointer, registers.stack_pointer
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_jsr() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0xca,
            "JSR",
            AddressingMode::Absolute([0x0a, 0x20]),
            jsr,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x20]);
        registers.stack_pointer = 0xff;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("JSR".to_owned(), log_line.mnemonic);
        assert_eq!(0x200a, registers.command_pointer);
        assert_eq!(0xfd, registers.stack_pointer);
        assert_eq!(
            vec![0x02, 0x10],
            memory.read(STACK_BASE_ADDR + 0xfe, 2).unwrap()
        );
    }
}
