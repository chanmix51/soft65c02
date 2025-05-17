use super::*;

pub fn cpy(
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
        .expect("CPY must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];

    registers.set_c_flag(registers.register_y >= byte);
    registers.set_z_flag(registers.register_y == byte);
    registers.set_n_flag(registers.register_y < byte);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "(0x{:02x})[Y=0x{:02x}][S={}]",
            byte,
            registers.register_y,
            registers.format_status()
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_cpy_immediate_greater() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xC0, "CPY", AddressingMode::Immediate([0x0a]), cpy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xC0, 0x0a]);
        registers.register_y = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("CPY".to_owned(), log_line.mnemonic);
        assert_eq!(0x1002, registers.command_pointer);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate: 2 cycles
        assert_eq!("#0x1000: (c0 0a)       CPY  #$0a     (#0x1001)  (0x0a)[Y=0x28][S=nv-BdizC][2]", log_line.to_string());
    }

    #[test]
    fn test_cpy_zero_page_equal() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xC4, "CPY", AddressingMode::ZeroPage([0x0a]), cpy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xC4, 0x0a]);
        registers.register_y = 0x0a;
        memory.write(0x0a, &[0x0a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (c4 0a)       CPY  $0a      (#0x000A)  (0x0a)[Y=0x0a][S=nv-BdiZC][3]", log_line.to_string());
    }

    #[test]
    fn test_cpy_absolute_less() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xCC, "CPY", AddressingMode::Absolute([0x00, 0x20]), cpy);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xCC, 0x00, 0x20]);
        registers.register_y = 0x01;
        memory.write(0x2000, &[0x0a]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (cc 00 20)    CPY  $2000    (#0x2000)  (0x0a)[Y=0x01][S=Nv-Bdizc][4]", log_line.to_string());
    }
}
