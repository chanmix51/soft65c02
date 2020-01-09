use super::*;

pub fn txs(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;

    registers.status_register = registers.register_x;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("[X=0x{:02x}][S={}]", registers.register_x, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_txs() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "TXS", AddressingMode::Implied, txs);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8a]);
        registers.register_x = 0x83;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("TXS".to_owned(), log_line.mnemonic);
        assert_eq!(0x83, registers.status_register);
        assert!(registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert!(registers.c_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.i_flag_is_set());
        assert!(!registers.d_flag_is_set());
        assert!(!registers.b_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
    }
}
