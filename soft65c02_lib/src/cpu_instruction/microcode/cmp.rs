use super::*;

pub fn cmp(
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
        .expect("cmp must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    let byte = memory.read(target_address, 1)?[0];

    registers.set_c_flag(registers.accumulator >= byte);
    registers.set_z_flag(registers.accumulator == byte);
    registers.set_n_flag(registers.accumulator < byte);
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
    fn test_cmp_immediate_greater() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xC9, "CMP", AddressingMode::Immediate([0x0a]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xC9, 0x0a]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("CMP".to_owned(), log_line.mnemonic);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (c9 0a)       CMP  #$0a     (#0x1001)  (0x0a)[A=0x28][S=nv-BdizC][2]", log_line.to_string());
    }

    #[test]
    fn test_cmp_zero_page_equal() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xC5, "CMP", AddressingMode::ZeroPage([0x0a]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xC5, 0x0a]);
        registers.accumulator = 0x0a;
        memory.write(0x0a, &[0x0a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (c5 0a)       CMP  $0a      (#0x000A)  (0x0a)[A=0x0a][S=nv-BdiZC][3]", log_line.to_string());
    }

    #[test]
    fn test_cmp_absolute_less() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xCD, "CMP", AddressingMode::Absolute([0x00, 0x20]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xCD, 0x00, 0x20]);
        registers.accumulator = 0x01;
        memory.write(0x2000, &[0x0a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (cd 00 20)    CMP  $2000    (#0x2000)  (0x0a)[A=0x01][S=Nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_cmp_absolute_x_no_boundary() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xDD, "CMP", AddressingMode::AbsoluteXIndexed([0x00, 0x20]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xDD, 0x00, 0x20]);
        registers.register_x = 0x05;
        registers.accumulator = 0x28;
        memory.write(0x2005, &[0x0a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(4, log_line.cycles); // Absolute,X: 4 cycles (no page boundary crossed)
        assert_eq!("#0x1000: (dd 00 20)    CMP  $2000,X  (#0x2005)  (0x0a)[A=0x28][S=nv-BdizC][4]", log_line.to_string());
    }

    #[test]
    fn test_cmp_absolute_x_boundary() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xDD, "CMP", AddressingMode::AbsoluteXIndexed([0xF0, 0x20]), cmp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xDD, 0xF0, 0x20]);
        registers.register_x = 0x15; // Cross page boundary: 0x20F0 + 0x15 = 0x2105
        registers.accumulator = 0x28;
        memory.write(0x2105, &[0x0a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(5, log_line.cycles); // Absolute,X: 5 cycles (page boundary crossed)
        assert_eq!("#0x1000: (dd f0 20)    CMP  $20F0,X  (#0x2105)  (0x0a)[A=0x28][S=nv-BdizC][5]", log_line.to_string());
    }
}
