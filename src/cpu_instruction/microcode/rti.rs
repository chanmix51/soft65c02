use super::*;

pub fn rti(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let status = registers.stack_pull(memory)?;
    registers.set_status_register(status);
    let cp_lo = registers.stack_pull(memory)?;
    let cp_hi = registers.stack_pull(memory)?;
    registers.command_pointer = (cp_hi as usize) << 8 | cp_lo as usize;

    Ok(LogLine::new(
        &cpu_instruction,
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

    #[test]
    fn test_rti() {
        let cpu_instruction =
            CPUInstruction::new(0x8000, 0xca, "RTI", AddressingMode::Implied, rti);
        let (mut memory, mut registers) = get_stuff(0x8000, vec![0x00]);
        memory
            .write(STACK_BASE_ADDR + 0xfd, vec![0b00110000, 0x01, 0x10])
            .unwrap();
        registers.stack_pointer = 0xfc;
        registers.set_z_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("RTI".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(0xff, registers.stack_pointer);
        assert!(!registers.z_flag_is_set());
    }
}
