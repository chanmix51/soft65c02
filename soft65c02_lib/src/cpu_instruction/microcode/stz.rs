use super::*;

pub fn stz(
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
        .expect("STZ instruction must have operands, crashing the application");

    memory.write(target_address, &[0x00])?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "0x00[S={}]",
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
    fn test_stz_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x64, "STZ", AddressingMode::ZeroPage([0x44]), stz);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x64, 0x44]);
        memory.write(0x44, &[0x42]).unwrap(); // Write non-zero value first
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("STZ".to_owned(), log_line.mnemonic);
        assert_eq!(0x00, memory.read(0x44, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (64 44)       STZ  $44      (#0x0044)  0x00[S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_stz_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x74, "STZ", AddressingMode::ZeroPageXIndexed([0x20]), stz);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x74, 0x20]);
        registers.register_x = 0x05; // Target address will be $25
        memory.write(0x25, &[0x42]).unwrap(); // Write non-zero value first
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, memory.read(0x25, 1).unwrap()[0]);
        assert_eq!(4, log_line.cycles); // Zero Page,X: 4 cycles
        assert_eq!("#0x1000: (74 20)       STZ  $20,X    (#0x0025)  0x00[S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_stz_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9C, "STZ", AddressingMode::Absolute([0x00, 0x44]), stz);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9C, 0x00, 0x44]);
        memory.write(0x4400, &[0x42]).unwrap(); // Write non-zero value first
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, memory.read(0x4400, 1).unwrap()[0]);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (9c 00 44)    STZ  $4400    (#0x4400)  0x00[S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_stz_absolute_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9E, "STZ", AddressingMode::AbsoluteXIndexed([0x00, 0x44]), stz);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9E, 0x00, 0x44]);
        registers.register_x = 0x05; // Target address will be $4405
        memory.write(0x4405, &[0x42]).unwrap(); // Write non-zero value first
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, memory.read(0x4405, 1).unwrap()[0]);
        assert_eq!(5, log_line.cycles); // Absolute,X: 5 cycles
        assert_eq!("#0x1000: (9e 00 44)    STZ  $4400,X  (#0x4405)  0x00[S=nv-Bdizc][5]", log_line.to_string());
    }
}
