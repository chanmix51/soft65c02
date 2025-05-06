use super::*;

pub fn jmp(
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
        .expect("JMP must have an operand, crashing the application");

    registers.command_pointer = target_address;

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!("[CP=0x{:04x}][S={}]", registers.command_pointer, registers.format_status()),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_jmp_absolute() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x4C,
            "JMP",
            AddressingMode::Absolute([0x0a, 0x02]),
            jmp,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4C, 0x0a, 0x02]);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("JMP".to_owned(), log_line.mnemonic);
        assert_eq!(0x020a, registers.command_pointer);
        assert_eq!(3, log_line.cycles); // Absolute: 3 cycles
        assert_eq!("#0x1000: (4c 0a 02)    JMP  $020A    (#0x020A)  [CP=0x020a][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_jmp_indirect() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x6C,
            "JMP",
            AddressingMode::Indirect([0x20, 0x02]),
            jmp,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x6C, 0x20, 0x02]);
        memory.write(0x0220, &[0x34, 0x12]).unwrap(); // Target address at $0220 points to $1234
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("JMP".to_owned(), log_line.mnemonic);
        assert_eq!(0x1234, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Indirect: 6 cycles on 65C02 (5 on 6502)
        assert_eq!("#0x1000: (6c 20 02)    JMP  ($0220)  (#0x1234)  [CP=0x1234][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_jmp_indirect_page_boundary() {
        // Test the 65C02 fix for the 6502 JMP indirect bug at page boundaries
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x6C,
            "JMP",
            AddressingMode::Indirect([0xFF, 0x12]), // Address $12FF at page boundary
            jmp,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x6C, 0xFF, 0x12]);
        memory.write(0x12FF, &[0x34]).unwrap(); // Low byte at $12FF
        memory.write(0x1300, &[0x12]).unwrap(); // High byte at $1300 (65C02 fix, 6502 would read from $1200)
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x1234, registers.command_pointer); // Should read high byte from correct page ($1300)
        assert_eq!(6, log_line.cycles); // 6 cycles on 65C02 due to page boundary fix
        assert_eq!("#0x1000: (6c ff 12)    JMP  ($12FF)  (#0x1234)  [CP=0x1234][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_jmp_indirect_page_boundary_bug_fix() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x6C,
            "JMP",
            AddressingMode::Indirect([0xFF, 0x12]), // $12FF - page boundary address
            jmp,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x6C, 0xFF, 0x12]);
        
        // Set up both possible high bytes:
        // - $1200 (where 6502 would incorrectly read from)
        // - $1300 (where 65C02 correctly reads from)
        memory.write(0x12FF, &[0x34]).unwrap(); // Low byte at $12FF
        memory.write(0x1200, &[0xBB]).unwrap(); // Wrong high byte (6502 bug would read this)
        memory.write(0x1300, &[0x12]).unwrap(); // Correct high byte (65C02 reads this)

        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();

        // Should jump to $1234 (using high byte from $1300)
        // NOT $BB34 (which would happen if reading high byte from $1200)
        assert_eq!(0x1234, registers.command_pointer, 
            "Should read high byte from $1300, not $1200 like the 6502 bug");
        assert_eq!(6, log_line.cycles, "Should take 6 cycles due to page boundary fix");
        assert_eq!("#0x1000: (6c ff 12)    JMP  ($12FF)  (#0x1234)  [CP=0x1234][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_jmp_indirect_indexed() {
        let cpu_instruction = CPUInstruction::new(
            0x1000,
            0x7C,
            "JMP",
            AddressingMode::AbsoluteXIndexedIndirect([0x20, 0x02]),
            jmp,
        );
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x7C, 0x20, 0x02]);
        registers.register_x = 0x02; // Target address will be at $0222
        memory.write(0x0222, &[0x34, 0x12]).unwrap(); // Target address stored at $0222
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x1234, registers.command_pointer);
        assert_eq!(6, log_line.cycles); // Indirect Indexed: 6 cycles
        assert_eq!("#0x1000: (7c 20 02)    JMP  ($0220,X)(#0x1234)  [CP=0x1234][S=nv-Bdizc][6]", log_line.to_string());
    }
}
