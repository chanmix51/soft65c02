use super::*;

pub fn inc(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let mut byte = match resolution.target_address {
        Some(addr) => memory.read(addr, 1)?[0],
        None       => registers.accumulator,
    };

    if byte == 0xff {
        byte = 0;
        registers.set_z_flag(true);
    } else {
        byte += 1;
        registers.set_z_flag(false);
    }
    registers.set_n_flag(byte & 0b10000000 != 0);

    let outcome = match resolution.target_address {
        Some(addr) => {
            memory.write(addr, vec![byte])?;
            format!("0x{:02x}[S={}]", byte, registers.format_status())
        },
        None       => {
            registers.accumulator = byte;
            format!("[A=0x{:02x}][S={}]", byte, registers.format_status())
        },
    };
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            outcome,
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_inc() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "INC", AddressingMode::ZeroPage([0x0a]), inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        memory.write(0x0a, vec![0x28]).unwrap();
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("INC".to_owned(), log_line.mnemonic);
        assert_eq!(0x29, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_inc_acc() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "INC", AddressingMode::Accumulator, inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("INC".to_owned(), log_line.mnemonic);
        assert_eq!(0x29, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_inc_with_z_flag() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "INC", AddressingMode::Accumulator, inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0xff;
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }

    #[test]
    fn test_inc_with_n_flag() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "INC", AddressingMode::Accumulator, inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0xf7;
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0xf8, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
