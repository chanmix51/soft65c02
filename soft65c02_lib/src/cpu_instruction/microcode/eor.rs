use super::*;

pub fn eor(
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
        .expect("No operand given to EOR instruction, crashing application.");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    let byte = memory.read(target_address, 1)?[0];
    registers.accumulator ^= byte;
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
    fn test_eor() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x49, "EOR", AddressingMode::Immediate([0x0a]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x49, 0x0a]);
        registers.accumulator = 0x02;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("EOR".to_owned(), log_line.mnemonic);
        assert_eq!(0x08, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (49 0a)       EOR  #$0a     (#0x1001)  (0x0a)[A=0x08][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_eor_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x4D, "EOR", AddressingMode::Absolute([0x00, 0x20]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4D, 0x00, 0x20]);
        registers.accumulator = 0x02;
        memory.write(0x2000, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x08, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (4d 00 20)    EOR  $2000    (#0x2000)  (0x0a)[A=0x08][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_eor_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x5D, "EOR", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x5D, 0xFF, 0x10]);
        registers.register_x = 0x01;
        registers.accumulator = 0x02;
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x08, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (5d ff 10)    EOR  $10FF,X  (#0x1100)  (0x0a)[A=0x08][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_eor_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x45, "EOR", AddressingMode::ZeroPage([0x20]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x45, 0x20]);
        registers.accumulator = 0x02;
        memory.write(0x20, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x08, registers.accumulator);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (45 20)       EOR  $20      (#0x0020)  (0x0a)[A=0x08][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_eor_with_z() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x49, "EOR", AddressingMode::Immediate([0x0a]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x49, 0x0a]);
        registers.accumulator = 0x0a;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (49 0a)       EOR  #$0a     (#0x1001)  (0x0a)[A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_eor_with_n() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x49, "EOR", AddressingMode::Immediate([0x0a]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x49, 0x0a]);
        registers.accumulator = 0xfa;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xf0, registers.accumulator);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (49 0a)       EOR  #$0a     (#0x1001)  (0x0a)[A=0xf0][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_eor_indirect_y() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x51, "EOR", AddressingMode::ZeroPageIndirectYIndexed([0x20]), eor);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x51, 0x20]);
        registers.register_y = 0x01;
        registers.accumulator = 0x02;
        memory.write(0x20, &[0xFF, 0x10]).unwrap();
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x08, registers.accumulator);
        assert_eq!(6, log_line.cycles); // Indirect,Y with page cross: 5 + 1 cycles
        assert_eq!("#0x1000: (51 20)       EOR  ($20),Y  (#0x1100)  (0x0a)[A=0x08][S=nv-Bdizc][6]", log_line.to_string());
    }
}
