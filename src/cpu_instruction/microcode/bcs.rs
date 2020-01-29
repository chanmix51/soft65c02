use super::*;

pub fn bcs(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    if registers.c_flag_is_set() {
        registers.command_pointer = resolve_relative(
            cpu_instruction.address,
            cpu_instruction.addressing_mode.get_operands()[0]
        ).expect("Could not resolve relative address for BCS");
    } else {
        registers.command_pointer += 1 + resolution.operands.len();
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
    fn test_bcs() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "BCS", AddressingMode::Relative(0x1000, [0x0a]), bcs);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BCS".to_owned(), log_line.mnemonic);
        assert!(registers.c_flag_is_set());
        assert_eq!(0x100c, registers.command_pointer);
    }

    #[test]
    fn test_bcs_with_c_clear() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "BCS", AddressingMode::Relative(0x1000, [0x0a]), bcs);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.set_c_flag(false);
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }
}
