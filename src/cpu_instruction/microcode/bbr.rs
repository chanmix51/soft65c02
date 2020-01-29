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
    let byte = memory.read(target_address, 1)?[0];
    let mut bit = 0b00000001;
    (0..cpu_instruction.opcode >> 4).for_each(|_| bit = bit << 1);

    if byte & bit != 0 {
        registers.command_pointer += 1 + resolution.operands.len();
    } else {
        registers.command_pointer = resolve_relative(
            cpu_instruction.address + 1,
            cpu_instruction.addressing_mode.get_operands()[1]
        ).expect("Could not resolve relative address for BBR");
    }

    Ok(LogLine::new(
        &cpu_instruction,
        resolution,
        format!("[CP=0x{:04X}]", registers.command_pointer),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bbr0() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x0f, "BBR0", AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0xfe]), bbr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x0f, 0x0a, 0xfe]);
        memory.write(0x000a, &vec![0x01]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BBR0".to_owned(), log_line.mnemonic);
        assert_eq!(0x1003, registers.command_pointer);
    }

    #[test]
    fn test_bbr7() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x7f, "BBR7", AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0xfe]), bbr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x7f, 0x0a, 0xfe]);
        memory.write(0x000a, &vec![0x80]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BBR7".to_owned(), log_line.mnemonic);
        assert_eq!(0x1003, registers.command_pointer);
    }

    #[test]
    fn test_branching_bbr3() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3f, "BBR3", AddressingMode::ZeroPageRelative(0x1000, [0x0a, 0x09]), bbr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3f, 0x0a, 0x09]);
        memory.write(0x000a, &vec![0xf7]).unwrap();
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x100c, registers.command_pointer);
    }
}

