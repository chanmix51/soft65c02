use super::*;

pub fn lda(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
            
    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);
            
    let target_address = resolution
        .target_address
        .expect("LDA instruction must have operands, crashing the application");

    registers.accumulator = memory.read(target_address, 1)?[0];
    registers.set_n_flag(registers.accumulator & 0b10000000 != 0);
    registers.set_z_flag(registers.accumulator == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[A=0x{:02x}][S={}]",
            registers.accumulator,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_lda() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA5, "LDA", AddressingMode::ZeroPage([0x0a]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA5, 0x0a]);
        registers.accumulator = 0x10;
        memory.write(0x000a, &[0x5a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("LDA".to_owned(), log_line.mnemonic);
        assert_eq!(0x5a, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (a5 0a)       LDA  $0a      (#0x000A)  [A=0x5a][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_lda_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA5, "LDA", AddressingMode::ZeroPage([0x0a]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA5, 0x0a]);
        registers.accumulator = 0x10;
        memory.write(0x000a, &[0x80]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (a5 0a)       LDA  $0a      (#0x000A)  [A=0x80][S=Nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_lda_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA5, "LDA", AddressingMode::ZeroPage([0x0a]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA5, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (a5 0a)       LDA  $0a      (#0x000A)  [A=0x00][S=nv-BdiZc][3]", log_line.to_string());
    }

    #[test]
    fn test_lda_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBD, "LDA", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBD, 0xFF, 0x10]);
        registers.register_x = 0x01; // This will cause page crossing: $10FF + $01 = $1100
        memory.write(0x1100, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (bd ff 10)    LDA  $10FF,X  (#0x1100)  [A=0x42][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_lda_absolute_x_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBD, "LDA", AddressingMode::AbsoluteXIndexed([0x50, 0x10]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBD, 0x50, 0x10]);
        registers.register_x = 0x01; // No page crossing: $1050 + $01 = $1051
        memory.write(0x1051, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Absolute,X without page cross: 4 cycles
        assert_eq!("#0x1000: (bd 50 10)    LDA  $1050,X  (#0x1051)  [A=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_lda_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA9, "LDA", AddressingMode::Immediate([0x42]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA9, 0x42]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (a9 42)       LDA  #$42     (#0x1001)  [A=0x42][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_lda_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA5, "LDA", AddressingMode::ZeroPage([0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA5, 0x44]);
        memory.write(0x44, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (a5 44)       LDA  $44      (#0x0044)  [A=0x42][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_lda_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB5, "LDA", AddressingMode::ZeroPageXIndexed([0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB5, 0x44]);
        registers.register_x = 0x02; // Target address will be $46
        memory.write(0x46, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Zero Page,X: 4 cycles
        assert_eq!("#0x1000: (b5 44)       LDA  $44,X    (#0x0046)  [A=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_lda_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xAD, "LDA", AddressingMode::Absolute([0x00, 0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xAD, 0x00, 0x44]);
        memory.write(0x4400, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (ad 00 44)    LDA  $4400    (#0x4400)  [A=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_lda_absolute_y_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB9, "LDA", AddressingMode::AbsoluteYIndexed([0xFF, 0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB9, 0xFF, 0x44]);
        registers.register_y = 0x01; // This will cause page crossing: $44FF + $01 = $4500
        memory.write(0x4500, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Absolute,Y with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (b9 ff 44)    LDA  $44FF,Y  (#0x4500)  [A=0x42][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_lda_absolute_y_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB9, "LDA", AddressingMode::AbsoluteYIndexed([0x50, 0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB9, 0x50, 0x44]);
        registers.register_y = 0x01; // No page crossing: $4450 + $01 = $4451
        memory.write(0x4451, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Absolute,Y without page cross: 4 cycles
        assert_eq!("#0x1000: (b9 50 44)    LDA  $4450,Y  (#0x4451)  [A=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_lda_indirect_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA1, "LDA", AddressingMode::ZeroPageXIndexedIndirect([0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA1, 0x44]);
        registers.register_x = 0x02; // Address to read from will be $46
        memory.write(0x46, &[0x00, 0x44]).unwrap(); // Points to $4400
        memory.write(0x4400, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(6, log_line.cycles); // Indirect,X: 6 cycles
        assert_eq!("#0x1000: (a1 44)       LDA  ($44,X)  (#0x4400)  [A=0x42][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_lda_indirect_y_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB1, "LDA", AddressingMode::ZeroPageIndirectYIndexed([0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB1, 0x44]);
        memory.write(0x44, &[0xFF, 0x44]).unwrap(); // Points to $44FF
        registers.register_y = 0x01; // This will cause page crossing: $44FF + $01 = $4500
        memory.write(0x4500, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(6, log_line.cycles); // Indirect,Y with page cross: 5 + 1 cycles
        assert_eq!("#0x1000: (b1 44)       LDA  ($44),Y  (#0x4500)  [A=0x42][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_lda_indirect_y_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB1, "LDA", AddressingMode::ZeroPageIndirectYIndexed([0x44]), lda);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB1, 0x44]);
        memory.write(0x44, &[0x50, 0x44]).unwrap(); // Points to $4450
        registers.register_y = 0x01; // No page crossing: $4450 + $01 = $4451
        memory.write(0x4451, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Indirect,Y without page cross: 5 cycles
        assert_eq!("#0x1000: (b1 44)       LDA  ($44),Y  (#0x4451)  [A=0x42][S=nv-Bdizc][5]", log_line.to_string());
    }
}
