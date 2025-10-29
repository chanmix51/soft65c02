use super::*;

pub fn sta(
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
        .expect("STA must have operands, crashing the application");

    // No cycle adjustments needed - table values are complete for write operations

    memory.write(target_address, &[registers.accumulator])?;
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("(0x{:02x})", registers.accumulator),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_sta_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x85, "STA", AddressingMode::ZeroPage([0x0a]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x85, 0x0a]);
        registers.accumulator = 0x10;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("STA".to_owned(), log_line.mnemonic);
        assert_eq!(0x10, memory.read(0x0a, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(3, cpu_instruction.cycles.get());
        assert_eq!("#0x1000: (85 0a)       STA  $0a      (#0x000A)  (0x10)[3]", log_line.to_string());
    }

    #[test]
    fn test_sta_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x8D, "STA", AddressingMode::Absolute([0x00, 0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x8D, 0x00, 0x20]);
        registers.accumulator = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x2000, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(4, cpu_instruction.cycles.get());
        assert_eq!("#0x1000: (8d 00 20)    STA  $2000    (#0x2000)  (0x42)[4]", log_line.to_string());
    }

    #[test]
    fn test_sta_zero_page_indirect() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x92, "STA", AddressingMode::ZeroPageIndirect([0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x92, 0x20]);
        memory.write(0x20, &[0x80, 0x40]).unwrap(); // Target address is 0x4080
        registers.accumulator = 0x37;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x37, memory.read(0x4080, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, cpu_instruction.cycles.get());
        assert_eq!("#0x1000: (92 20)       STA  ($20)    (#0x4080)  (0x37)[5]", log_line.to_string());
    }

    #[test]
    fn test_sta_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x95, "STA", AddressingMode::ZeroPageXIndexed([0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x95, 0x20]);
        registers.register_x = 0x05; // Write to 0x25
        registers.accumulator = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x25, 1).unwrap()[0]);
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(4, cpu_instruction.cycles.get());
        assert_eq!("#0x1000: (95 20)       STA  $20,X    (#0x0025)  (0x42)[4]", log_line.to_string());
    }

    #[test]
    fn test_sta_absolute_y() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x99, "STA", AddressingMode::AbsoluteYIndexed([0x00, 0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x99, 0x00, 0x20]);
        registers.register_y = 0x05; // Write to 0x2005
        registers.accumulator = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x2005, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(5, cpu_instruction.cycles.get());
        assert_eq!("#0x1000: (99 00 20)    STA  $2000,Y  (#0x2005)  (0x42)[5]", log_line.to_string());
    }

    #[test]
    fn test_sta_absolute_y_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x99, "STA", AddressingMode::AbsoluteYIndexed([0xFB, 0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x99, 0xFB, 0x20]);
        registers.register_y = 0x05; // Write to 0x2100 (page cross)
        registers.accumulator = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x2100, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(5, cpu_instruction.cycles.get()); // Write operations always take 5 cycles, no page cross penalty
        assert_eq!("#0x1000: (99 fb 20)    STA  $20FB,Y  (#0x2100)  (0x42)[5]", log_line.to_string());
    }

    #[test]
    fn test_sta_absolute_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9D, "STA", AddressingMode::AbsoluteXIndexed([0x00, 0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9D, 0x00, 0x20]);
        registers.register_x = 0x05; // Write to 0x2005
        registers.accumulator = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x2005, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(5, cpu_instruction.cycles.get()); // Absolute indexed write: 5 cycles
        assert_eq!("#0x1000: (9d 00 20)    STA  $2000,X  (#0x2005)  (0x42)[5]", log_line.to_string());
    }

    #[test]
    fn test_sta_absolute_x_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x9D, "STA", AddressingMode::AbsoluteXIndexed([0xFB, 0x20]), sta);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x9D, 0xFB, 0x20]);
        registers.register_x = 0x05; // Write to 0x2100 (page cross)
        registers.accumulator = 0x42;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x42, memory.read(0x2100, 1).unwrap()[0]);
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(5, cpu_instruction.cycles.get()); // Write operations always take 5 cycles, no page cross penalty
        assert_eq!("#0x1000: (9d fb 20)    STA  $20FB,X  (#0x2100)  (0x42)[5]", log_line.to_string());
    }
}
