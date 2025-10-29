use super::*;

pub fn inc(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    let mut byte = match resolution.target_address {
        Some(addr) => memory.read(addr, 1)?[0],
        None => registers.accumulator,
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
            memory.write(addr, &[byte])?;
            format!("(0x{:02x})[S={}]", byte, registers.format_status())
        }
        None => {
            registers.accumulator = byte;
            format!("[A=0x{:02x}][S={}]", byte, registers.format_status())
        }
    };
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        outcome,
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_inc_accumulator() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x1A, "INC", AddressingMode::Accumulator, inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x1A]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("INC".to_owned(), log_line.mnemonic);
        assert_eq!(0x29, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Accumulator mode: 2 cycles
        assert_eq!("#0x1000: (1a)          INC  A                   [A=0x29][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_inc_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xE6, "INC", AddressingMode::ZeroPage([0x0a]), inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xE6, 0x0a]);
        memory.write(0x0a, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("INC".to_owned(), log_line.mnemonic);
        assert_eq!(0x29, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // Zero page: 5 cycles
        assert_eq!("#0x1000: (e6 0a)       INC  $0a      (#0x000A)  (0x29)[S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_inc_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xF6, "INC", AddressingMode::ZeroPageXIndexed([0x20]), inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xF6, 0x20]);
        registers.register_x = 0x05; // Target address: 0x25
        memory.write(0x25, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x29, memory.read(0x25, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Zero page X: 6 cycles
        assert_eq!("#0x1000: (f6 20)       INC  $20,X    (#0x0025)  (0x29)[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_inc_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xEE, "INC", AddressingMode::Absolute([0x00, 0x20]), inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xEE, 0x00, 0x20]);
        memory.write(0x2000, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x29, memory.read(0x2000, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Absolute: 6 cycles
        assert_eq!("#0x1000: (ee 00 20)    INC  $2000    (#0x2000)  (0x29)[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_inc_absolute_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xFE, "INC", AddressingMode::AbsoluteXIndexed([0x00, 0x20]), inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xFE, 0x00, 0x20]);
        registers.register_x = 0x05; // Target address: 0x2005
        memory.write(0x2005, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x29, memory.read(0x2005, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(7, log_line.cycles); // Absolute X: 7 cycles
        assert_eq!("#0x1000: (fe 00 20)    INC  $2000,X  (#0x2005)  (0x29)[S=nv-Bdizc][7]", log_line.to_string());
    }

    #[test]
    fn test_inc_absolute_x_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xFE, "INC", AddressingMode::AbsoluteXIndexed([0xFB, 0x20]), inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xFE, 0xFB, 0x20]);
        registers.register_x = 0x05; // Target address: 0x2100 (page cross)
        memory.write(0x2100, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x29, memory.read(0x2100, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(7, log_line.cycles); // Absolute X: 7 cycles (no extra cycle for page cross on write)
        assert_eq!("#0x1000: (fe fb 20)    INC  $20FB,X  (#0x2100)  (0x29)[S=nv-Bdizc][7]", log_line.to_string());
    }

    #[test]
    fn test_inc_with_zero_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x1A, "INC", AddressingMode::Accumulator, inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x1A]);
        registers.accumulator = 0xFF;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (1a)          INC  A                   [A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_inc_with_negative_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x1A, "INC", AddressingMode::Accumulator, inc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x1A]);
        registers.accumulator = 0x7F;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (1a)          INC  A                   [A=0x80][S=Nv-Bdizc][2]", log_line.to_string());
    }
}
