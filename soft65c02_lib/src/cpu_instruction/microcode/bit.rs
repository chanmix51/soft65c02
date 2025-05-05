use super::*;

pub fn bit(
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
        .expect("BIT must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    let byte = memory.read(target_address, 1)?[0];
    registers.set_z_flag(registers.accumulator & byte == 0);

    /*
     * see http://forum.6502.org/viewtopic.php?f=2&t=2241&p=27243#p27239
     */
    match cpu_instruction.addressing_mode {
        AddressingMode::Immediate(_) => {}
        _ => {
            registers.set_v_flag(byte & 0b01000000 != 0);
            registers.set_n_flag(byte & 0b10000000 != 0);
        }
    };
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
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bit() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x89, "BIT", AddressingMode::Immediate([0x0a]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x89, 0x0a]);
        registers.accumulator = 0x03;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BIT".to_owned(), log_line.mnemonic);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (89 0a)       BIT  #$0a     (#0x1001)  (0x0a)[A=0x03][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_bit_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x2C, "BIT", AddressingMode::Absolute([0x00, 0x20]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x2C, 0x00, 0x20]);
        registers.accumulator = 0x03;
        memory.write(0x2000, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (2c 00 20)    BIT  $2000    (#0x2000)  (0x0a)[A=0x03][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_bit_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3C, "BIT", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3C, 0xFF, 0x10]);
        registers.register_x = 0x01;
        registers.accumulator = 0x03;
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (3c ff 10)    BIT  $10FF,X  (#0x1100)  (0x0a)[A=0x03][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_bit_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x24, "BIT", AddressingMode::ZeroPage([0x20]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x24, 0x20]);
        registers.accumulator = 0x03;
        memory.write(0x20, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (24 20)       BIT  $20      (#0x0020)  (0x0a)[A=0x03][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_bit_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x24, "BIT", AddressingMode::ZeroPage([0xa0]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x24, 0xa0]);
        registers.accumulator = 0x03;
        memory.write(0xa0, &[0xba]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (24 a0)       BIT  $a0      (#0x00A0)  (0xba)[A=0x03][S=Nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_bit_negative_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x89, "BIT", AddressingMode::Immediate([0xba]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x89, 0xba]);
        registers.accumulator = 0x03;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (89 ba)       BIT  #$ba     (#0x1001)  (0xba)[A=0x03][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_bit_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x89, "BIT", AddressingMode::Immediate([0x03]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x89, 0x03]);
        registers.accumulator = 0x04;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (89 03)       BIT  #$03     (#0x1001)  (0x03)[A=0x04][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_bit_overflow() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x24, "BIT", AddressingMode::ZeroPage([0xa0]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x24, 0xa0]);
        registers.accumulator = 0x03;
        memory.write(0xa0, &[0x4d]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (24 a0)       BIT  $a0      (#0x00A0)  (0x4d)[A=0x03][S=nV-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_bit_overflow_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x89, "BIT", AddressingMode::Immediate([0x4d]), bit);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x89, 0x4d]);
        registers.accumulator = 0x03;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (89 4d)       BIT  #$4d     (#0x1001)  (0x4d)[A=0x03][S=nv-Bdizc][2]", log_line.to_string());
    }
}
