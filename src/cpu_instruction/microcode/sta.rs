use super::*;

pub fn sta(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("STA must have operands, crashing the application");

    memory.write(target_address, vec![registers.accumulator]).unwrap();
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(&cpu_instruction, resolution, format!("[A=0x{:02x}]", registers.accumulator)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sta() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "STA", AddressingMode::ZeroPage([0x0a]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xa9, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("STA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, memory.read(0x0a, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
    }
}
