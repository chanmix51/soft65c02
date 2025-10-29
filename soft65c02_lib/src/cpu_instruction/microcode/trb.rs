use super::*;

pub fn trb(
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
        .expect("TRB must have operands, crashing the application");

    let mut byte = memory.read(target_address, 1)?[0];
    if byte & registers.accumulator != 0 {
        byte &= registers.accumulator ^ 0xff;
        memory.write(target_address, &[byte])?;
        registers.set_z_flag(false);
    } else {
        registers.set_z_flag(true);
    }
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("(0x{:02x})[S={}]", byte, registers.format_status()),
        registers,
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
    fn test_trb_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x14, "TRB", AddressingMode::ZeroPage([0x00]), trb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x14, 0x00]);
        memory.write(0x00, &[0xa6]).unwrap();
        registers.accumulator = 0x33;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TRB".to_owned(), log_line.mnemonic);
        assert_eq!(0x84, memory.read(0x00, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert_eq!(0x33, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // Zero Page: 5 cycles
        assert_eq!("#0x1000: (14 00)       TRB  $00      (#0x0000)  (0x84)[S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_trb_zero_page_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x14, "TRB", AddressingMode::ZeroPage([0x00]), trb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x14, 0x00]);
        memory.write(0x00, &[0xa6]).unwrap();
        registers.accumulator = 0x41;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xa6, memory.read(0x00, 1).unwrap()[0]);
        assert!(registers.z_flag_is_set());
        assert_eq!(0x41, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles);
        assert_eq!("#0x1000: (14 00)       TRB  $00      (#0x0000)  (0xa6)[S=nv-BdiZc][5]", log_line.to_string());
    }

    #[test]
    fn test_trb_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x1C, "TRB", AddressingMode::Absolute([0x00, 0x20]), trb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x1C, 0x00, 0x20]);
        memory.write(0x2000, &[0xa6]).unwrap();
        registers.accumulator = 0x33;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TRB".to_owned(), log_line.mnemonic);
        assert_eq!(0x84, memory.read(0x2000, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert_eq!(0x33, registers.accumulator);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Absolute: 6 cycles
        assert_eq!("#0x1000: (1c 00 20)    TRB  $2000    (#0x2000)  (0x84)[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_trb_absolute_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x1C, "TRB", AddressingMode::Absolute([0x00, 0x20]), trb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x1C, 0x00, 0x20]);
        memory.write(0x2000, &[0xa6]).unwrap();
        registers.accumulator = 0x41;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xa6, memory.read(0x2000, 1).unwrap()[0]);
        assert!(registers.z_flag_is_set());
        assert_eq!(0x41, registers.accumulator);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles);
        assert_eq!("#0x1000: (1c 00 20)    TRB  $2000    (#0x2000)  (0xa6)[S=nv-BdiZc][6]", log_line.to_string());
    }
}
