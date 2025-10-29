use super::*;

pub fn bbr(
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
        .expect("BBR must have operands, crashing the application");
    
    // Test the specified bit
    let byte = memory.read(target_address, 1)?[0];
    let mut bit = 0b00000001;
    (0..cpu_instruction.opcode >> 4).for_each(|_| bit <<= 1);

    // Branch if the bit is reset (0)
    if byte & bit != 0 {
        registers.command_pointer += 3; // Skip to next instruction (opcode + 2 operands)
    } else {
        registers.command_pointer = resolve_relative(
            cpu_instruction.address + 1,
            cpu_instruction.addressing_mode.get_operands()[1],
        )
        .expect("Could not resolve relative address for BBR");
    }

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[CP=0x{:04X}]", registers.command_pointer),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bbr0_not_branching() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x0f,
            "BBR0",
            AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0x09]),
            bbr,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x0f, 0x0a, 0x09]);
        memory.write(0x000a, &[0x01]).unwrap(); // Set bit 0, should not branch
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BBR0".to_owned(), log_line.mnemonic);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // BBR always takes 5 cycles
        assert_eq!("#0x1000: (0f 0a 09)    BBR0 $0a,$100B(#0x000A)  [CP=0x1003][5]", log_line.to_string());
    }

    #[test]
    fn test_bbr0_branching() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x0f,
            "BBR0",
            AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0x09]),
            bbr,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x0f, 0x0a, 0x09]);
        memory.write(0x000a, &[0xfe]).unwrap(); // Clear bit 0, should branch
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BBR0".to_owned(), log_line.mnemonic);
        assert_eq!(0x100c, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // BBR always takes 5 cycles
        assert_eq!("#0x1000: (0f 0a 09)    BBR0 $0a,$100B(#0x000A)  [CP=0x100C][5]", log_line.to_string());
    }

    #[test]
    fn test_bbr7_not_branching() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x7f,
            "BBR7",
            AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0x09]),
            bbr,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x7f, 0x0a, 0x09]);
        memory.write(0x000a, &[0x80]).unwrap(); // Set bit 7, should not branch
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BBR7".to_owned(), log_line.mnemonic);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // BBR always takes 5 cycles
        assert_eq!("#0x1000: (7f 0a 09)    BBR7 $0a,$100B(#0x000A)  [CP=0x1003][5]", log_line.to_string());
    }

    #[test]
    fn test_bbr7_branching() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x7f,
            "BBR7",
            AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0x09]),
            bbr,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x7f, 0x0a, 0x09]);
        memory.write(0x000a, &[0x7f]).unwrap(); // Clear bit 7, should branch
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BBR7".to_owned(), log_line.mnemonic);
        assert_eq!(0x100c, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // BBR always takes 5 cycles
        assert_eq!("#0x1000: (7f 0a 09)    BBR7 $0a,$100B(#0x000A)  [CP=0x100C][5]", log_line.to_string());
    }
}
