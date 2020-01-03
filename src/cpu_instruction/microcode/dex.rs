use super::*;

pub fn dex(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    if registers.register_x != 0 {
        registers.register_x -= 1;

        if registers.register_x == 0 {
            registers.status_register |= 0b00000010;
        } else {
            registers.status_register &= 0b01111101;
        }
    } else {
        registers.register_x = 0xff;
        registers.status_register |= 0b10000000;
    }

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(&cpu_instruction, resolution, format!("[X=0x{:02x}]", registers.register_x)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_dex() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEX", AddressingMode::Implied, dex);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.register_x = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("DEX".to_owned(), log_line.mnemonic);
        assert_eq!(0x0f, registers.register_x);
        assert_eq!(0b00000000, registers.status_register & 0b10000010);
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_dex_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEX", AddressingMode::Implied, dex);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("DEX".to_owned(), log_line.mnemonic);
        assert_eq!(0xff, registers.register_x);
        assert_eq!(0b10000000, registers.status_register & 0b10000010);
        assert_eq!(0x1001, registers.command_pointer);
    }
}
