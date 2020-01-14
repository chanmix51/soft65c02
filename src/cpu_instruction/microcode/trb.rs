use super::*;

pub fn trb(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> Result<LogLine> {
    let resolution = cpu_instruction.addressing_mode
        .solve(registers.command_pointer, memory, registers)?;
    let target_address = resolution.target_address
        .expect("TRB must have operands, crashing the application");

    let byte = memory.read(target_address, 1)?[0];
    let res = if byte & registers.accumulator != 0 {
        registers.set_z_flag(true);
        let neg = registers.accumulator ^ 0xff; // negates all bits
        let res = byte & neg;
        memory.write(target_address, vec![res])?;
        res
    } else {
        registers.set_z_flag(false);
        byte
    };
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(
        LogLine::new(
            &cpu_instruction,
            resolution,
            format!("0x{:02x}[S={}]", res, registers.format_status())
        )
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_trb() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "TRB", AddressingMode::ZeroPage([0x0a]), trb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8a, 0x0a, 0x02]);
        memory.write(0x0a, vec![0xff]).unwrap();
        registers.accumulator = 0x81;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!("TRB".to_owned(), log_line.mnemonic);
        assert_eq!(0b01111110, memory.read(0x0a, 1).unwrap()[0]);
        assert!(registers.z_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }

    #[test]
    fn test_trb_with_no_z_flag() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "TRB", AddressingMode::ZeroPage([0x0a]), trb);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8a, 0x0a, 0x02]);
        memory.write(0x0a, vec![0x0f]).unwrap();
        registers.register_y = 0x80;
        let log_line = cpu_instruction.execute(&mut memory, &mut registers).unwrap();
        assert_eq!(0x0f, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
    }
}
