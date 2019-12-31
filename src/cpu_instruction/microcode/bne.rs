use super::*;

pub fn bne(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("BNE must have operands, crashing the application");

    if registers.status_register & 0b01000000 != 0 {
        registers.command_pointer = target_address;
    } else {
        registers.command_pointer += 2;
    }

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
    fn test_bne() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "BNE", AddressingMode::Relative([0x0a]), bne);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a, 0x02]);
        registers.status_register = 0b01011000;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("BNE".to_owned(), log_line.mnemonic);
        assert_eq!(0x100a, registers.command_pointer);
    }
}
