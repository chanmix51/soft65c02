use super::*;

pub fn tsx(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    registers.register_x = registers.stack_pointer;
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
    fn test_tsx() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBA, "TSX", AddressingMode::Implied, tsx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBA]);
        registers.stack_pointer = 0x7e;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("TSX".to_owned(), log_line.mnemonic);
        assert_eq!(0x7e, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // TSX takes 2 cycles
        assert_eq!("#0x1000: (ba)          TSX                      [X=0x7e][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_tsx_with_n_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBA, "TSX", AddressingMode::Implied, tsx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBA]);
        registers.stack_pointer = 0xfe;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xfe, registers.register_x);
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (ba)          TSX                      [X=0xfe][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_tsx_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xBA, "TSX", AddressingMode::Implied, tsx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xBA]);
        registers.stack_pointer = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.register_x);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (ba)          TSX                      [X=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }
}
