use super::*;

pub fn tsb(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution
        .target_address
        .expect("TSB must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];
    registers.set_z_flag(byte & registers.accumulator == 0);
    let res = byte | registers.accumulator;
    memory.write(target_address, &[res])?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("(0x{:02x})[S={}]", res, registers.format_status()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    /*
     * Examples come from http://www.6502.org/tutorials/65c02opcodes.html
     */
    #[test]
    fn test_tsb_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x04, "TSB", AddressingMode::ZeroPage([0x0a]), tsb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x04, 0x0a]);
        memory.write(0x0a, &[0xa6]).unwrap();
        registers.accumulator = 0x33;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TSB".to_owned(), log_line.mnemonic);
        assert_eq!(0xb7, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // Zero Page: 5 cycles
        assert_eq!("#0x1000: (04 0a)       TSB  $0a      (#0x000A)  (0xb7)[S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_tsb_zero_page_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x04, "TSB", AddressingMode::ZeroPage([0x0a]), tsb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x04, 0x0a]);
        memory.write(0x0a, &[0xa6]).unwrap();
        registers.accumulator = 0x41;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xe7, memory.read(0x0a, 1).unwrap()[0]);
        assert!(registers.z_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles);
        assert_eq!("#0x1000: (04 0a)       TSB  $0a      (#0x000A)  (0xe7)[S=nv-BdiZc][5]", log_line.to_string());
    }

    #[test]
    fn test_tsb_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x0C, "TSB", AddressingMode::Absolute([0x00, 0x20]), tsb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x0C, 0x00, 0x20]);
        memory.write(0x2000, &[0xa6]).unwrap();
        registers.accumulator = 0x33;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TSB".to_owned(), log_line.mnemonic);
        assert_eq!(0xb7, memory.read(0x2000, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Absolute: 6 cycles
        assert_eq!("#0x1000: (0c 00 20)    TSB  $2000    (#0x2000)  (0xb7)[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_tsb_absolute_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x0C, "TSB", AddressingMode::Absolute([0x00, 0x20]), tsb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x0C, 0x00, 0x20]);
        memory.write(0x2000, &[0xa6]).unwrap();
        registers.accumulator = 0x41;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xe7, memory.read(0x2000, 1).unwrap()[0]);
        assert!(registers.z_flag_is_set());
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles);
        assert_eq!("#0x1000: (0c 00 20)    TSB  $2000    (#0x2000)  (0xe7)[S=nv-BdiZc][6]", log_line.to_string());
    }
}
