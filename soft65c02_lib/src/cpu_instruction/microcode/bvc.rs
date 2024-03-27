use super::*;

pub fn bvc(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    if registers.v_flag_is_set() {
        registers.command_pointer += 1 + resolution.operands.len();
    } else {
        registers.command_pointer = resolve_relative(
            cpu_instruction.address,
            cpu_instruction.addressing_mode.get_operands()[0]
        ).expect("Could not resolve relative address for BVC");
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
    fn test_bvc() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "BVC", AddressingMode::Relative(0x1000, [0x0a]), bvc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.set_v_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BVC".to_owned(), log_line.mnemonic);
        assert!(registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_bvc_with_v_clear() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "BVC", AddressingMode::Relative(0x1000, [0x0a]), bvc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.set_v_flag(false);
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x100c, registers.command_pointer);
    }
}
