use super::*;

pub fn rti(
    memory: &mut Memory,
    registers: &mut Registers,
    cpu_instruction: &CPUInstruction,
) -> Result<LogLine> {
    let resolution =
        cpu_instruction
            .addressing_mode
            .solve(registers.command_pointer, memory, registers)?;

    let status = registers.stack_pull(memory)?;
    registers.set_status_register(status);
    let cp_lo = registers.stack_pull(memory)?;
    let cp_hi = registers.stack_pull(memory)?;
    registers.command_pointer = (cp_hi as usize) << 8 | cp_lo as usize;

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "[CP=0x{:04X}][SP=0x{:02x}][S={}]",
            registers.command_pointer,
            registers.stack_pointer,
            registers.format_status()
        ),
        registers,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;
    use crate::STACK_BASE_ADDR;

    #[test]
    fn test_rti_basic() {
        let cpu_instruction =
            CPUInstruction::new(0x8000, 0x40, "RTI", AddressingMode::Implied, rti);
        let (mut memory, mut registers) = get_stuff(0x8000, vec![0x40]);
        
        // Push test values onto stack (in reverse order as they'll be pulled):
        // - Status flags: 0b00110000 (no flags set)
        // - Return address: $1234
        memory.write(STACK_BASE_ADDR + 0xfd, &[0b00110000, 0x34, 0x12]).unwrap();
        registers.stack_pointer = 0xfc;
        
        // Set some flags that should be overwritten
        registers.set_z_flag(true);
        registers.set_n_flag(true);
        registers.set_c_flag(true);
        
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
            
        assert_eq!("RTI".to_owned(), log_line.mnemonic);
        assert_eq!(0x1234, registers.command_pointer, "Should return to exact address from stack");
        assert_eq!(0xff, registers.stack_pointer, "Should pull 3 bytes from stack");
        assert_eq!(6, log_line.cycles, "RTI should take 6 cycles");
        
        // Verify flags were restored from stack
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert!(!registers.d_flag_is_set());
        assert!(!registers.i_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.c_flag_is_set());
        
        assert_eq!("#0x8000: (40)          RTI                      [CP=0x1234][SP=0xff][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_rti_all_flags_set() {
        let cpu_instruction =
            CPUInstruction::new(0x8000, 0x40, "RTI", AddressingMode::Implied, rti);
        let (mut memory, mut registers) = get_stuff(0x8000, vec![0x40]);
        
        // Set all flags in status byte (0xFF)
        memory.write(STACK_BASE_ADDR + 0xfd, &[0xFF, 0x34, 0x12]).unwrap();
        registers.stack_pointer = 0xfc;
        
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
            
        // Verify all flags were set
        assert!(registers.n_flag_is_set());
        assert!(registers.v_flag_is_set());
        assert!(registers.d_flag_is_set());
        assert!(registers.i_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(registers.c_flag_is_set());
        
        assert_eq!(6, log_line.cycles, "RTI should take 6 cycles");
        assert_eq!("#0x8000: (40)          RTI                      [CP=0x1234][SP=0xff][S=NV-BDIZC][6]", log_line.to_string());
    }

    #[test]
    fn test_rti_interrupt_sequence() {
        // This test verifies RTI works correctly after an interrupt sequence
        let cpu_instruction =
            CPUInstruction::new(0x8000, 0x40, "RTI", AddressingMode::Implied, rti);
        let (mut memory, mut registers) = get_stuff(0x8000, vec![0x40]);
        
        // Simulate interrupt sequence:
        // 1. Push return address $1234
        // 2. Push processor status with I flag set
        registers.command_pointer = 0x1234;
        registers.set_i_flag(true);
        let status = registers.get_status_register();
        
        // Stack should contain status then return address
        memory.write(STACK_BASE_ADDR + 0xfd, &[status, 0x34, 0x12]).unwrap();
        registers.stack_pointer = 0xfc;
        
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
            
        assert_eq!(0x1234, registers.command_pointer, "Should restore exact interrupt return address");
        assert!(registers.i_flag_is_set(), "Should restore I flag from interrupt");
        assert_eq!(6, log_line.cycles, "RTI should take 6 cycles");
        assert_eq!("#0x8000: (40)          RTI                      [CP=0x1234][SP=0xff][S=nv-BdIzc][6]", log_line.to_string());
    }
}
