use super::*;

pub fn sbc(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode.solve(registers.command_pointer, memory, registers)?;
    let target_address = match resolution.target_address {
        Some(v) => v,
        None => panic!("Ooops no target address from the addressing mode resolver."),
    };

    let byte = memory.read(target_address, 1).unwrap()[0];
    registers.accumulator -= byte;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine {
        address:    cpu_instruction.address,
        opcode:     cpu_instruction.opcode,
        mnemonic:   cpu_instruction.mnemonic.clone(),
        resolution: resolution,
        is_simulated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sbc() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "sbc", AddressingMode::Immediate([0x0a]), sbc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("sbc".to_owned(), log_line.mnemonic);
        assert_eq!(0x1e, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
    }
}
