use super::*;

pub fn dec(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let byte = match resolution.target_address {
        Some(addr)  => memory.read(addr, 1)?[0],
        None        => registers.accumulator,
    };
    let (res, _) = byte.overflowing_sub(1);

    registers.set_z_flag(res == 0);
    registers.set_n_flag(res & 0b10000000 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    let outcome = match resolution.target_address {
        Some(addr)  => {
            memory.write(addr, vec![res])?;
            format!("0x{:02x}", res)
        },
        None        => {
            registers.accumulator = res;
            format!("[A=0x{:02x}]", res)
        },
    };

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("{}[S={}]", outcome, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_dec() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEC", AddressingMode::Accumulator, dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("DEC".to_owned(), log_line.mnemonic);
        assert_eq!(0x0f, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_dec_memory() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEC", AddressingMode::ZeroPage([0xa0]), dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        memory.write(0xa0, vec![0x10]).unwrap();
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x0f, memory.read(0xa0, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_dec_when_zero() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEC", AddressingMode::Accumulator, dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.accumulator = 0x00;
        registers.set_c_flag(false);
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0xff, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.c_flag_is_set()); // C flag is NOT affected.
        assert!(registers.n_flag_is_set());
    }

    #[test]
    fn test_dec_when_one() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEC", AddressingMode::Implied, dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.accumulator = 0x01;
        let _log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
    }
}

