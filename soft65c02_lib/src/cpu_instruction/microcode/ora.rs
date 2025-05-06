use super::*;

pub fn ora(
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
        .expect("ORA must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    let byte = memory.read(target_address, 1)?[0];
    registers.accumulator |= byte;
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "(0x{:02x})[A=0x{:02x}][S={}]",
            byte,
            registers.accumulator,
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
    fn test_ora() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x09, "ORA", AddressingMode::Immediate([0x0a]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x09, 0x0a]);
        registers.accumulator = 0x22;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("ORA".to_owned(), log_line.mnemonic);
        assert_eq!(0x2A, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (09 0a)       ORA  #$0a     (#0x1001)  (0x0a)[A=0x2a][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_ora_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x0D, "ORA", AddressingMode::Absolute([0x00, 0x20]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x0D, 0x00, 0x20]);
        registers.accumulator = 0x22;
        memory.write(0x2000, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x2A, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (0d 00 20)    ORA  $2000    (#0x2000)  (0x0a)[A=0x2a][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_ora_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x1D, "ORA", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x1D, 0xFF, 0x10]);
        registers.register_x = 0x01;
        registers.accumulator = 0x22;
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x2A, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (1d ff 10)    ORA  $10FF,X  (#0x1100)  (0x0a)[A=0x2a][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_ora_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x05, "ORA", AddressingMode::ZeroPage([0x20]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x05, 0x20]);
        registers.accumulator = 0x22;
        memory.write(0x20, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x2A, registers.accumulator);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (05 20)       ORA  $20      (#0x0020)  (0x0a)[A=0x2a][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_ora_set_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x09, "ORA", AddressingMode::Immediate([0x00]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x09, 0x00]);
        registers.accumulator = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (09 00)       ORA  #$00     (#0x1001)  (0x00)[A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_ora_set_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x09, "ORA", AddressingMode::Immediate([0x80]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x09, 0x80]);
        registers.accumulator = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (09 80)       ORA  #$80     (#0x1001)  (0x80)[A=0x80][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_ora_indirect_y() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x11, "ORA", AddressingMode::ZeroPageIndirectYIndexed([0x20]), ora);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x11, 0x20]);
        registers.register_y = 0x01;
        registers.accumulator = 0x22;
        memory.write(0x20, &[0xFF, 0x10]).unwrap();
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x2A, registers.accumulator);
        assert_eq!(6, log_line.cycles); // Indirect,Y with page cross: 5 + 1 cycles
        assert_eq!("#0x1000: (11 20)       ORA  ($20),Y  (#0x1100)  (0x0a)[A=0x2a][S=nv-Bdizc][6]", log_line.to_string());
    }
}
