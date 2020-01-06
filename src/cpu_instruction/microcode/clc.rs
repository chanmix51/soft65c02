use super::*;

pub fn clc(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    registers.set_c_flag(false);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(&cpu_instruction, resolution, format!("[S={}]", registers.format_status())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_clc() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "CLC", AddressingMode::Implied, clc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.set_c_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("CLC".to_owned(), log_line.mnemonic);
        assert_eq!(0x1001, registers.command_pointer);
        assert!(!registers.c_flag_is_set());
    }
}

