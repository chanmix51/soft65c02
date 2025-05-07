use super::*;

/// # BRK
///
/// Generate a [soft interrupt](http://6502.org/tutorials/interrupts.html#2.2).
///
/// When the processor encounters that instruction, it acts like a hardware
/// interrupt occures (with subtle differences).
///
/// * Command Pointer register is pushed to the stack pointing 2 bytes after the
/// BRK instruction.
/// * Status register is pushed to the stack with the B flag set .
/// * Status register I flag is set, D flag is cleared.
///
pub fn brk(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let bytes = usize::to_le_bytes(registers.command_pointer + 2); // 1 extra padding byte
    registers.stack_push(memory, bytes[1])?;
    registers.stack_push(memory, bytes[0])?;
    registers.stack_push(memory, registers.get_status_register())?;
    registers.command_pointer = little_endian(memory.read(INTERRUPT_VECTOR_ADDR, 2)?);
    registers.set_i_flag(true);
    registers.set_d_flag(false);

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
    fn test_brk() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x00, "BRK", AddressingMode::Implied, brk);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x00]);
        memory.write(0xfffe, &[0x00, 0xf0]).unwrap();
        registers.stack_pointer = 0xff;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BRK".to_owned(), log_line.mnemonic);
        assert_eq!(0xf000, registers.command_pointer);
        assert_eq!(0xfc, registers.stack_pointer);
        assert_eq!(
            vec![0b00110000, 0x02, 0x10],
            memory.read(STACK_BASE_ADDR + 0xfd, 3).unwrap()
        );
        assert!(registers.i_flag_is_set());
        assert!(!registers.d_flag_is_set());
        assert_eq!(7, log_line.cycles); // Implied: 7 cycles
        assert_eq!("#0x1000: (00)          BRK                      [CP=0xF000][SP=0xfc][S=nv-BdIzc][7]", log_line.to_string());
    }
}
