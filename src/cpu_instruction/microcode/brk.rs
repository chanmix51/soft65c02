use super::*;

/* BRK
 * Generate a soft interrupt
 *
 * @see http://6502.org/tutorials/interrupts.html#2.2
 */

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

    Ok(LogLine::new(
        &cpu_instruction,
        resolution,
        format!(
            "[CP=0x{:04X}][SP=0x{:02x}]",
            registers.command_pointer, registers.stack_pointer
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_brk() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "BRK", AddressingMode::Implied, brk);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x00]);
        memory.write(0xfffe, vec![0x00, 0xf0]).unwrap();
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
    }
}
