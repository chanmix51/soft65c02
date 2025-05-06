use super::*;

/// # AND | Logical AND operation
///
/// Performs a AND operation between the Accumulator and the specified target.
/// Result is stored in the Accumulator. Affects flags NC.
///
pub fn and(
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
        .expect("AND must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    let byte = memory.read(target_address, 1)?[0];
    registers.accumulator &= byte;
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
    fn test_and_immediate() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x29, "AND", AddressingMode::Immediate([0x0a]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x00, 0x0a, 0x02]);
        registers.accumulator = 0x22;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("AND".to_owned(), log_line.mnemonic);
        assert_eq!(0x02, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (29 0a)       AND  #$0a     (#0x1001)  (0x0a)[A=0x02][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_and_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3D, "AND", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3D, 0xFF, 0x10]);
        registers.register_x = 0x01;
        registers.accumulator = 0x22;
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (3d ff 10)    AND  $10FF,X  (#0x1100)  (0x0a)[A=0x02][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_and_absolute_x_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x3D, "AND", AddressingMode::AbsoluteXIndexed([0x50, 0x10]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x3D, 0x50, 0x10]);
        registers.register_x = 0x01;
        registers.accumulator = 0x22;
        memory.write(0x1051, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert_eq!(4, log_line.cycles); // Absolute,X without page cross: 4 cycles
        assert_eq!("#0x1000: (3d 50 10)    AND  $1050,X  (#0x1051)  (0x0a)[A=0x02][S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_and_indirect_y_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x31, "AND", AddressingMode::ZeroPageIndirectYIndexed([0x20]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x31, 0x20]);
        registers.register_y = 0x01;
        registers.accumulator = 0x22;
        memory.write(0x20, &[0xFF, 0x10]).unwrap();
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert_eq!(6, log_line.cycles); // Indirect,Y with page cross: 5 + 1 cycles
        assert_eq!("#0x1000: (31 20)       AND  ($20),Y  (#0x1100)  (0x0a)[A=0x02][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_and_indirect_y_no_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x31, "AND", AddressingMode::ZeroPageIndirectYIndexed([0x20]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x31, 0x20]);
        registers.register_y = 0x01;
        registers.accumulator = 0x22;
        memory.write(0x20, &[0x50, 0x10]).unwrap();
        memory.write(0x1051, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Indirect,Y without page cross: 5 cycles
        assert_eq!("#0x1000: (31 20)       AND  ($20),Y  (#0x1051)  (0x0a)[A=0x02][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_and_set_zero() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x29, "AND", AddressingMode::Immediate([0x55]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x55, 0x02]);
        registers.accumulator = 0xaa;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (29 55)       AND  #$55     (#0x1001)  (0x55)[A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_and_set_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x29, "AND", AddressingMode::Immediate([0xaa]), and);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xaa]);
        registers.accumulator = 0xd5;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x80, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (29 aa)       AND  #$aa     (#0x1001)  (0xaa)[A=0x80][S=Nv-Bdizc][2]", log_line.to_string());
    }
}
