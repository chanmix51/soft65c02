use super::*;

pub fn bra(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let original_cp = registers.command_pointer;

    registers.command_pointer = resolve_relative(
        cpu_instruction.address,
        cpu_instruction.addressing_mode.get_operands()[0],
    )
    .expect("Could not resolve relative address for BRA");

    // For BRA, only add the page crossing cycle if needed
    // The branch cycle is already included in the instruction table
    if (original_cp + 2) & 0xFF00 != registers.command_pointer & 0xFF00 {
        cpu_instruction.cycles.set(cpu_instruction.cycles.get() + 1);
    }

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[CP=0x{:04X}]", registers.command_pointer),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bra_no_page_cross() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x80,
            "BRA",
            AddressingMode::Relative(0x1000, [0x0a]),
            bra,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x80, 0x0a]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BRA".to_owned(), log_line.mnemonic);
        assert_eq!(0x100c, registers.command_pointer);
        assert_eq!(3, cpu_instruction.cycles.get(), "BRA without page cross should take 3 cycles");
        assert_eq!("#0x1000: (80 0a)       BRA  $100C               [CP=0x100C][3]", log_line.to_string());
    }

    #[test]
    fn test_bra_page_cross() {
        let cpu_instruction = CPUInstruction::new(
            0x10f0,
            0x80,
            "BRA",
            AddressingMode::Relative(0x10f0, [0x20]),
            bra,
        );
        let (mut memory, mut registers) = get_stuff(0x10f0, vec![0x80, 0x20]);
        registers.command_pointer = 0x10f0;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x1112, registers.command_pointer);
        assert_eq!(4, cpu_instruction.cycles.get(), "BRA with page cross should take 4 cycles");
        assert_eq!("#0x10F0: (80 20)       BRA  $1112               [CP=0x1112][4]", log_line.to_string());
    }
}
