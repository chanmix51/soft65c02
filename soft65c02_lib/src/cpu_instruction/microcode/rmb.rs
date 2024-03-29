use super::*;

pub fn rmb(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;
    let addr = resolution
        .target_address
        .expect("RMB must have operands, crashing the application");
    let byte = memory.read(addr, 1)?[0];
    let mut bit = 0b00000001;
    (0..cpu_instruction.opcode >> 4).for_each(|_| bit <<= 1);
    let bit = 0b11111111 ^ bit;
    let byte = byte & bit;
    memory.write(addr, &[byte])?;

    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("(0x{:02x})", byte),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_rmb0() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x07, "RMB0", AddressingMode::ZeroPage([0x0a]), rmb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x07, 0x0a]);
        memory.write(0x0a, &[0xff]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("RMB0".to_owned(), log_line.mnemonic);
        assert_eq!(0xfe, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(
            format!("#0x1000: (07 0a)       RMB0 $0a      (#0x000A)  (0xfe)"),
            format!("{}", log_line)
        );
    }

    #[test]
    fn test_rmb7() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x77, "RMB7", AddressingMode::ZeroPage([0x0a]), rmb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x77, 0x0a]);
        memory.write(0x0a, &[0xff]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("RMB7".to_owned(), log_line.mnemonic);
        assert_eq!(0x7f, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(
            format!("#0x1000: (77 0a)       RMB7 $0a      (#0x000A)  (0x7f)"),
            format!("{}", log_line)
        );
    }
}
