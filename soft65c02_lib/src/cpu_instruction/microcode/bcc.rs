use super::*;

pub fn bcc(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let original_cp = registers.command_pointer;

    if registers.c_flag_is_set() {
        registers.command_pointer += 1 + resolution.operands.len();
    } else {
        registers.command_pointer = resolve_relative(
            cpu_instruction.address,
            cpu_instruction.addressing_mode.get_operands()[0],
        )
        .expect("Could not resolve relative address for BCC");
        
        // Add cycles after we know the branch was taken
        cpu_instruction.add_branch_cycles(registers, original_cp);
    }

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[CP=0x{:04X}]", registers.command_pointer),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bcc_not_branching() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x90,
            "BCC",
            AddressingMode::Relative(0x1000, [0x0a]),
            bcc,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x90, 0x0a]);
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("BCC".to_owned(), log_line.mnemonic);
        assert!(registers.c_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles);
        assert_eq!("#0x1000: (90 0a)       BCC  $100C               [CP=0x1002][2]", log_line.to_string());
    }

    #[test]
    fn test_bcc_branching_no_page_cross() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x90,
            "BCC",
            AddressingMode::Relative(0x1000, [0x0a]),
            bcc,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x90, 0x0a]);
        registers.set_c_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x100c, registers.command_pointer);
        assert_eq!(3, log_line.cycles);
        assert_eq!("#0x1000: (90 0a)       BCC  $100C               [CP=0x100C][3]", log_line.to_string());
    }

    #[test]
    fn test_bcc_branching_page_cross() {
        let cpu_instruction = CPUInstruction::new(
            0x10f0,
            0x90,
            "BCC",
            AddressingMode::Relative(0x10f0, [0x20]),
            bcc,
        );
        let (mut memory, mut registers) = get_stuff(0x10f0, vec![0x90, 0x20]);
        registers.set_c_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1112, registers.command_pointer);
        assert_eq!(4, log_line.cycles);
        assert_eq!("#0x10F0: (90 20)       BCC  $1112               [CP=0x1112][4]", log_line.to_string());
    }
}
