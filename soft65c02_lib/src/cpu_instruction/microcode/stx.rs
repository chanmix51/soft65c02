use super::*;

pub fn stx(
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
        .expect("STX instruction must have operands, crashing the application");

    memory.write(target_address, &[registers.register_x])?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "0x{:02x}[S={}]",
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
    fn test_stx_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x86, "STX", AddressingMode::ZeroPage([0x44]), stx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x86, 0x44]);
        registers.register_x = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("STX".to_owned(), log_line.mnemonic);
        assert_eq!(0x42, memory.read(0x44, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (86 44)       STX  $44      (#0x0044)  0x42[S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_stx_zero_page_y() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x96, "STX", AddressingMode::ZeroPageYIndexed([0x20]), stx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x96, 0x20]);
        registers.register_x = 0x42;
        registers.register_y = 0x05; // Target address will be $25
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x25, 1).unwrap()[0]);
        assert_eq!(4, log_line.cycles); // Zero Page,Y: 4 cycles
        assert_eq!("#0x1000: (96 20)       STX  $20,Y    (#0x0025)  0x42[S=nv-Bdizc][4]", log_line.to_string());
    }

    #[test]
    fn test_stx_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x8E, "STX", AddressingMode::Absolute([0x00, 0x44]), stx);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8E, 0x00, 0x44]);
        registers.register_x = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x4400, 1).unwrap()[0]);
        assert_eq!(4, log_line.cycles); // Absolute: 4 cycles
        assert_eq!("#0x1000: (8e 00 44)    STX  $4400    (#0x4400)  0x42[S=nv-Bdizc][4]", log_line.to_string());
    }
}
