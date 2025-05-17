use super::*;

pub fn smb(
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
        .expect("SMB expects an operand, crashing the application");
    let byte = memory.read(addr, 1)?[0];

    let mut bit = 0b00000001;
    (0..(cpu_instruction.opcode >> 4) - 8).for_each(|_| bit <<= 1);
    let byte = byte | bit;
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
    fn test_smb0() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x87, "SMB0", AddressingMode::ZeroPage([0x0a]), smb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x87, 0x0a]);
        memory.write(0x0a, &[0x00]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("SMB0".to_owned(), log_line.mnemonic);
        assert_eq!(0x01, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles);
        assert_eq!("#0x1000: (87 0a)       SMB0 $0a      (#0x000A)  (0x01)[5]", log_line.to_string());
    }

    #[test]
    fn test_smb7() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0xf7, "SMB7", AddressingMode::ZeroPage([0x0a]), smb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xf7, 0x0a]);
        memory.write(0x0a, &[0x00]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("SMB7".to_owned(), log_line.mnemonic);
        assert_eq!(0x80, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles);
        assert_eq!("#0x1000: (f7 0a)       SMB7 $0a      (#0x000A)  (0x80)[5]", log_line.to_string());
    }
}
