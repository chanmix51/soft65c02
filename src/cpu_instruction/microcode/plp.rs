use super::*;

pub fn plp(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    let status = registers.stack_pull(memory)?;
    registers.set_status_register(status);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!(
                "[SP=0x{:02x}][S={}]",
                registers.stack_pointer,
                registers.format_status()
                )
            )
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_plp() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0x08, "PLP", AddressingMode::Implied, plp);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x08, 0x0a]);
        registers.set_z_flag(true);
        registers.set_d_flag(true);
        registers.set_c_flag(false);
        registers.set_n_flag(false);
        memory.write(STACK_BASE_ADDR + 0xff, vec![registers.get_status_register()]);
        registers.stack_pointer = 0xfe;
        registers.set_z_flag(false);
        registers.set_d_flag(false);
        registers.set_c_flag(true);
        registers.set_n_flag(true);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("PLP".to_owned(), log_line.mnemonic);
        assert!(registers.d_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0xff, registers.stack_pointer);
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(format!("#0x1000: (08)          PLP                      [SP=0xff][S=nv-BDiZc]"), format!("{}", log_line));
    }
}



