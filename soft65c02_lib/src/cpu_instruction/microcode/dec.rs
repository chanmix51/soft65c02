use super::*;

pub fn dec(
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

    let (res, _) = byte.overflowing_sub(1);

    registers.set_z_flag(res == 0);
    registers.set_n_flag(res & 0b10000000 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    let outcome = match resolution.target_address {
        Some(addr) => {
            memory.write(addr, &[res])?;
            format!("0x{:02x}", res)
        }
        None => {
            registers.accumulator = res;
            format!("[A=0x{:02x}]", res)
        }
    };

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("{}[S={}]", outcome, registers.format_status()),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_dec_accumulator() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3A, "DEC", AddressingMode::Accumulator, dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3A]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("DEC".to_owned(), log_line.mnemonic);
        assert_eq!(0x0f, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Accumulator mode: 2 cycles
        assert_eq!("#0x1000: (3a)          DEC  A                   [A=0x0f][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_dec_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xC6, "DEC", AddressingMode::ZeroPage([0x0a]), dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xC6, 0x0a]);
        memory.write(0x0a, &[0x10]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x0f, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // Zero page: 5 cycles
        assert_eq!("#0x1000: (c6 0a)       DEC  $0a      (#0x000A)  0x0f[S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_dec_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xD6, "DEC", AddressingMode::ZeroPageXIndexed([0x20]), dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xD6, 0x20]);
        registers.register_x = 0x05; // Target address: 0x25
        memory.write(0x25, &[0x10]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x0f, memory.read(0x25, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Zero page X: 6 cycles
        assert_eq!("#0x1000: (d6 20)       DEC  $20,X    (#0x0025)  0x0f[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_dec_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xCE, "DEC", AddressingMode::Absolute([0x00, 0x20]), dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xCE, 0x00, 0x20]);
        memory.write(0x2000, &[0x10]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x0f, memory.read(0x2000, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Absolute: 6 cycles
        assert_eq!("#0x1000: (ce 00 20)    DEC  $2000    (#0x2000)  0x0f[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_dec_absolute_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xDE, "DEC", AddressingMode::AbsoluteXIndexed([0x00, 0x20]), dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xDE, 0x00, 0x20]);
        registers.register_x = 0x05; // Target address: 0x2005
        memory.write(0x2005, &[0x10]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x0f, memory.read(0x2005, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(7, log_line.cycles); // Absolute X: 7 cycles
        assert_eq!("#0x1000: (de 00 20)    DEC  $2000,X  (#0x2005)  0x0f[S=nv-Bdizc][7]", log_line.to_string());
    }

    #[test]
    fn test_dec_absolute_x_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xDE, "DEC", AddressingMode::AbsoluteXIndexed([0xFB, 0x20]), dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xDE, 0xFB, 0x20]);
        registers.register_x = 0x05; // Target address: 0x2100 (page cross)
        memory.write(0x2100, &[0x10]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x0f, memory.read(0x2100, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(7, log_line.cycles); // Absolute X: 7 cycles (no extra cycle for page cross on write)
        assert_eq!("#0x1000: (de fb 20)    DEC  $20FB,X  (#0x2100)  0x0f[S=nv-Bdizc][7]", log_line.to_string());
    }

    #[test]
    fn test_dec_with_zero_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3A, "DEC", AddressingMode::Accumulator, dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3A]);
        registers.accumulator = 0x01;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (3a)          DEC  A                   [A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_dec_with_negative_result() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3A, "DEC", AddressingMode::Accumulator, dec);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3A]);
        registers.accumulator = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xFF, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (3a)          DEC  A                   [A=0xff][S=Nv-Bdizc][2]", log_line.to_string());
    }
}
