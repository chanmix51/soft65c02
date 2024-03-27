use super::*;

pub fn asl(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let byte = match resolution.target_address {
        Some(addr) => memory.read(addr, 1)?[0],
        None => registers.accumulator,
    };

    let (res, _carry) = byte.overflowing_shl(1);
    registers.set_c_flag(res < byte); // maybe not ideal but has_carry stays false.
    registers.set_z_flag(res == 0);
    registers.set_n_flag(res & 0x80 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    let outcome = match resolution.target_address {
        Some(addr) => {
            memory.write(addr, &vec![res])?;
            format!("0x{:02x}[S={}]", res, registers.format_status())
        }
        None => {
            registers.accumulator = res;
            format!("[A=0x{:02x}][S={}]", res, registers.format_status())
        }
    };

    Ok(LogLine::new(&cpu_instruction, resolution, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_asl() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ASL", AddressingMode::ZeroPage([0x0a]), asl);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        memory.write(0x0a, &vec![0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("ASL".to_owned(), log_line.mnemonic);
        assert_eq!(0x50, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_asl_acc() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ASL", AddressingMode::Accumulator, asl);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x50, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn test_asl_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ASL", AddressingMode::Accumulator, asl);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0x00;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
    }

    #[test]
    fn test_asl_with_c_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ASL", AddressingMode::Accumulator, asl);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0x81;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(registers.c_flag_is_set());
    }

    #[test]
    fn test_asl_with_n_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xca, "ASL", AddressingMode::Accumulator, asl);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xe8, 0x0a, 0x02]);
        registers.accumulator = 0x47;
        let _log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x8e, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(registers.n_flag_is_set());
    }
}
