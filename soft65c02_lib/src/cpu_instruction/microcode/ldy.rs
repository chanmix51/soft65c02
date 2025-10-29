use super::*;

pub fn ldy(
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
        .expect("LDY instruction must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    registers.register_y = memory.read(target_address, 1)?[0];
    registers.set_n_flag(registers.register_y & 0b10000000 != 0);
    registers.set_z_flag(registers.register_y == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[Y=0x{:02x}][S={}]",
            registers.register_y,
            registers.format_status()
        ),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_ldy_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA0, "LDY", AddressingMode::Immediate([0x0a]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA0, 0x0a]);
        registers.register_y = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("LDY".to_owned(), log_line.mnemonic);
        assert_eq!(0x0a, registers.register_y);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (a0 0a)       LDY  #$0a     (#0x1001)  [Y=0x0a][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_ldy_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA4, "LDY", AddressingMode::ZeroPage([0x44]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA4, 0x44]);
        memory.write(0x44, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_y);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (a4 44)       LDY  $44      (#0x0044)  [Y=0x42][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_ldy_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB4, "LDY", AddressingMode::ZeroPageXIndexed([0x20]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB4, 0x20]);
        registers.register_x = 0x05; // Target address will be $25
        memory.write(0x25, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_y);
        assert_eq!(4, log_line.cycles); // Zero Page,X: 4 cycles
        assert_eq!("#0x1000: (b4 20)       LDY  $20,X    (#0x0025)  [Y=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ldy_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xAC, "LDY", AddressingMode::Absolute([0x00, 0x44]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xAC, 0x00, 0x44]);
        memory.write(0x4400, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_y);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (ac 00 44)    LDY  $4400    (#0x4400)  [Y=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ldy_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBC, "LDY", AddressingMode::AbsoluteXIndexed([0xFF, 0x44]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBC, 0xFF, 0x44]);
        registers.register_x = 0x01; // This will cause page crossing: $44FF + $01 = $4500
        memory.write(0x4500, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_y);
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (bc ff 44)    LDY  $44FF,X  (#0x4500)  [Y=0x42][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_ldy_absolute_x_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBC, "LDY", AddressingMode::AbsoluteXIndexed([0x50, 0x44]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBC, 0x50, 0x44]);
        registers.register_x = 0x01; // No page crossing: $4450 + $01 = $4451
        memory.write(0x4451, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_y);
        assert_eq!(4, log_line.cycles); // Absolute,X without page cross: 4 cycles
        assert_eq!("#0x1000: (bc 50 44)    LDY  $4450,X  (#0x4451)  [Y=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ldy_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA0, "LDY", AddressingMode::Immediate([0x8a]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA0, 0x8a]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x8a, registers.register_y);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (a0 8a)       LDY  #$8a     (#0x1001)  [Y=0x8a][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_ldy_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA0, "LDY", AddressingMode::Immediate([0x00]), ldy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA0, 0x00]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_y);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (a0 00)       LDY  #$00     (#0x1001)  [Y=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }
}
