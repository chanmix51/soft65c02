use super::*;

/// # LSR - Logical Shift Right
///
/// On the 65C02 (unlike the 6502):
/// - LSR absolute,X takes 6 cycles when no page boundary is crossed
/// - LSR absolute,X takes 7 cycles when a page boundary is crossed
/// - On the 6502, LSR absolute,X always takes 7 cycles regardless of page crossing
///
/// This implementation follows the 65C02 behavior.
/// See http://www.6502.org/tutorials/65c02opcodes.html
pub fn lsr(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    let byte = match resolution.target_address {
        Some(addr) => memory.read(addr, 1)?[0],
        None => registers.accumulator,
    };

    let (res, _) = byte.overflowing_shr(1);
    registers.set_c_flag(byte & 1 == 1);
    registers.set_z_flag(res == 0);
    registers.set_n_flag(false); // bit 7 is always 0 when shifting right
    registers.command_pointer += 1 + resolution.operands.len();

    let outcome = match resolution.target_address {
        Some(addr) => {
            memory.write(addr, &[res])?;
            format!("0x{:02x}[S={}]", res, registers.format_status())
        }
        None => {
            registers.accumulator = res;
            format!("[A=0x{:02x}][S={}]", res, registers.format_status())
        }
    };

    Ok(LogLine::new(cpu_instruction, resolution, outcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_lsr() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x46, "LSR", AddressingMode::ZeroPage([0x0a]), lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x46, 0x0a]);
        memory.write(0x0a, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("LSR".to_owned(), log_line.mnemonic);
        assert_eq!(0x14, memory.read(0x0a, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(5, log_line.cycles); // Zero Page: 5 cycles
        assert_eq!("#0x1000: (46 0a)       LSR  $0a      (#0x000A)  0x14[S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_lsr_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x5E, "LSR", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x5E, 0xFF, 0x10]);
        registers.register_x = 0x01; // This will cause page crossing: $10FF + $01 = $1100
        memory.write(0x1100, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x14, memory.read(0x1100, 1).unwrap()[0]);
        assert_eq!(7, log_line.cycles); // Absolute,X: 6 cycles + 1 for page crossing
        assert_eq!("#0x1000: (5e ff 10)    LSR  $10FF,X  (#0x1100)  0x14[S=nv-Bdizc][7]", log_line.to_string());
    }

    #[test]
    fn test_lsr_absolute_x_without_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x5E, "LSR", AddressingMode::AbsoluteXIndexed([0x80, 0x10]), lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x5E, 0x80, 0x10]);
        registers.register_x = 0x01; // No page crossing: $1080 + $01 = $1081
        memory.write(0x1081, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x14, memory.read(0x1081, 1).unwrap()[0]);
        assert_eq!(6, log_line.cycles); // Absolute,X: 6 cycles (no page crossing)
        assert_eq!("#0x1000: (5e 80 10)    LSR  $1080,X  (#0x1081)  0x14[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_lsr_acc() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x4A, "LSR", AddressingMode::Accumulator, lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4A]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x14, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Accumulator: 2 cycles
        assert_eq!("#0x1000: (4a)          LSR  A                   [A=0x14][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_lsr_with_z_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x4A, "LSR", AddressingMode::Accumulator, lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4A]);
        registers.accumulator = 0x00;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Accumulator: 2 cycles
        assert_eq!("#0x1000: (4a)          LSR  A                   [A=0x00][S=nv-BdiZc][2]", log_line.to_string());
    }

    #[test]
    fn test_lsr_with_c_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x4A, "LSR", AddressingMode::Accumulator, lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4A]);
        registers.accumulator = 0x81;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x40, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(registers.c_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Accumulator: 2 cycles
        assert_eq!("#0x1000: (4a)          LSR  A                   [A=0x40][S=nv-BdizC][2]", log_line.to_string());
    }

    #[test]
    fn test_lsr_with_n_flag() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x4A, "LSR", AddressingMode::Accumulator, lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4A]);
        registers.accumulator = 0xfe;
        registers.set_n_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x7f, registers.accumulator);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert_eq!(0x1001, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Accumulator: 2 cycles
        assert_eq!("#0x1000: (4a)          LSR  A                   [A=0x7f][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_lsr_absolute() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x4E, "LSR", AddressingMode::Absolute([0x00, 0x20]), lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4E, 0x00, 0x20]);
        memory.write(0x2000, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x14, memory.read(0x2000, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1003, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Absolute: 6 cycles
        assert_eq!("#0x1000: (4e 00 20)    LSR  $2000    (#0x2000)  0x14[S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_lsr_zero_page_x() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x56, "LSR", AddressingMode::ZeroPageXIndexed([0x20]), lsr);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x56, 0x20]);
        registers.register_x = 0x05; // Target address will be $25
        memory.write(0x25, &[0x28]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x14, memory.read(0x25, 1).unwrap()[0]);
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.c_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Zero Page,X: 6 cycles
        assert_eq!("#0x1000: (56 20)       LSR  $20,X    (#0x0025)  0x14[S=nv-Bdizc][6]", log_line.to_string());
    }
}
