use super::*;

pub fn ldx(
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
        .expect("LDX instruction must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    registers.register_x = memory.read(target_address, 1)?[0];
    registers.set_n_flag(registers.register_x & 0b10000000 != 0);
    registers.set_z_flag(registers.register_x == 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[X=0x{:02x}][S={}]",
            registers.register_x,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_ldx_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA2, "LDX", AddressingMode::Immediate([0x0a]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA2, 0x0a]);
        registers.register_x = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("LDX".to_owned(), log_line.mnemonic);
        assert_eq!(0x0a, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (a2 0a)       LDX  #$0a     (#0x1001)  [X=0x0a][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_ldx_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA6, "LDX", AddressingMode::ZeroPage([0x44]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA6, 0x44]);
        memory.write(0x44, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_x);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (a6 44)       LDX  $44      (#0x0044)  [X=0x42][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_ldx_zero_page_y() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xB6, "LDX", AddressingMode::ZeroPageYIndexed([0x20]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xB6, 0x20]);
        registers.register_y = 0x05; // Target address will be $25
        memory.write(0x25, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_x);
        assert_eq!(4, log_line.cycles); // Zero Page,Y: 4 cycles
        assert_eq!("#0x1000: (b6 20)       LDX  $20,Y    (#0x0025)  [X=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ldx_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xAE, "LDX", AddressingMode::Absolute([0x00, 0x44]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xAE, 0x00, 0x44]);
        memory.write(0x4400, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_x);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (ae 00 44)    LDX  $4400    (#0x4400)  [X=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ldx_absolute_y_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBE, "LDX", AddressingMode::AbsoluteYIndexed([0xFF, 0x44]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBE, 0xFF, 0x44]);
        registers.register_y = 0x01; // This will cause page crossing: $44FF + $01 = $4500
        memory.write(0x4500, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_x);
        assert_eq!(5, log_line.cycles); // Absolute,Y with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (be ff 44)    LDX  $44FF,Y  (#0x4500)  [X=0x42][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_ldx_absolute_y_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBE, "LDX", AddressingMode::AbsoluteYIndexed([0x50, 0x44]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBE, 0x50, 0x44]);
        registers.register_y = 0x01; // No page crossing: $4450 + $01 = $4451
        memory.write(0x4451, &[0x42]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, registers.register_x);
        assert_eq!(4, log_line.cycles); // Absolute,Y without page cross: 4 cycles
        assert_eq!("#0x1000: (be 50 44)    LDX  $4450,Y  (#0x4451)  [X=0x42][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ldx_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA2, "LDX", AddressingMode::Immediate([0x8a]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA2, 0x8a]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x8a, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (a2 8a)       LDX  #$8a     (#0x1001)  [X=0x8a][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_ldx_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xA2, "LDX", AddressingMode::Immediate([0x00]), ldx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xA2, 0x00]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (a2 00)       LDX  #$00     (#0x1001)  [X=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }
}
