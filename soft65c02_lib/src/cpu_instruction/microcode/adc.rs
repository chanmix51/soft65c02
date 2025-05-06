use super::*;

/// # ADC - Add with carry
///
/// The 65C02 has only one instruction for addition, an addition with carry.
/// Note: the formula for the oVerflow bit comes from
/// http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
/// Method to handle the decimal mode comes from
/// http://www.6502.org/tutorials/decimal_mode.html
///
/// On the 65C02 (unlike the 6502):
/// - In decimal mode, N, V, and Z flags are valid
/// - Decimal mode takes one extra cycle compared to binary mode
/// See http://www.6502.org/tutorials/65c02opcodes.html
pub fn adc(
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
        .expect("ADC must have operands, crashing the application");

    // Add extra cycle for page boundary crossing in indexed addressing modes
    cpu_instruction.adjust_base_cycles(registers, memory);

    // Add extra cycle for decimal mode on 65C02
    if registers.d_flag_is_set() {
        cpu_instruction.cycles.set(cpu_instruction.cycles.get() + 1);
    }

    let byte = memory.read(target_address, 1)?[0];
    let a = registers.accumulator;

    if registers.d_flag_is_set() {
        let carry = if registers.c_flag_is_set() { 1 } else { 0 };
        let low1 = a & 0x0F;
        let low2 = byte & 0x0F;
        let subres = low1 + low2 + carry;
        let sublow = subres % 10;
        let carry = if subres == sublow { 0 } else { 1 };
        let hi1 = a >> 4;
        let hi2 = byte >> 4;
        let subres = hi1 + hi2 + carry;
        let subhi = subres % 10;
        registers.set_c_flag(subhi != subres);
        registers.accumulator = (subhi << 4) | sublow;
    } else {
        let (res, c) = byte.overflowing_add(if registers.c_flag_is_set() { 1 } else { 0 });
        let (res, has_carry) = a.overflowing_add(res);
        registers.accumulator = res;
        registers.set_c_flag(has_carry | c);
    }
    registers.set_z_flag(registers.accumulator == 0);
    registers.set_n_flag(registers.accumulator & 0x80 != 0);
    registers.set_v_flag((a ^ registers.accumulator) & (byte ^ registers.accumulator) & 0x80 != 0);
    registers.command_pointer += 1 + resolution.operands.len();

    Ok(LogLine::new(
        cpu_instruction,
        resolution,
        format!(
            "(0x{:02x})[A=0x{:02x}][S={}]",
            byte,
            registers.accumulator,
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
    fn test_adc() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!("ADC".to_owned(), log_line.mnemonic);
        assert_eq!(0x32, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(0x1002, registers.command_pointer);
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 0a)       ADC  #$0a     (#0x1001)  (0x0a)[A=0x32][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_adc_absolute_x_with_page_cross() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x7D, "ADC", AddressingMode::AbsoluteXIndexed([0xFF, 0x10]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x7D, 0xFF, 0x10]);
        registers.register_x = 0x01;
        registers.accumulator = 0x28;
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x32, registers.accumulator);
        assert_eq!(5, log_line.cycles); // Absolute,X with page cross: 4 + 1 cycles
        assert_eq!("#0x1000: (7d ff 10)    ADC  $10FF,X  (#0x1100)  (0x0a)[A=0x32][S=nv-Bdizc][5]", log_line.to_string());
    }

    #[test]
    fn test_adc_zero_page() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x65, "ADC", AddressingMode::ZeroPage([0x20]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x65, 0x20]);
        registers.accumulator = 0x28;
        memory.write(0x20, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x32, registers.accumulator);
        assert_eq!(3, log_line.cycles); // Zero Page: 3 cycles
        assert_eq!("#0x1000: (65 20)       ADC  $20      (#0x0020)  (0x0a)[A=0x32][S=nv-Bdizc][3]", log_line.to_string());
    }

    #[test]
    fn test_adc_indirect_y() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x71, "ADC", AddressingMode::ZeroPageIndirectYIndexed([0x20]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x71, 0x20]);
        registers.register_y = 0x01;
        registers.accumulator = 0x28;
        memory.write(0x20, &[0xFF, 0x10]).unwrap();
        memory.write(0x1100, &[0x0A]).unwrap();
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x32, registers.accumulator);
        assert_eq!(6, log_line.cycles); // Indirect,Y with page cross: 5 + 1 cycles
        assert_eq!("#0x1000: (71 20)       ADC  ($20),Y  (#0x1100)  (0x0a)[A=0x32][S=nv-Bdizc][6]", log_line.to_string());
    }

    #[test]
    fn test_adc_with_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0x28;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x33, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 0a)       ADC  #$0a     (#0x1001)  (0x0a)[A=0x33][S=nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_adc_set_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0xf8;
        registers.set_c_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x02, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 0a)       ADC  #$0a     (#0x1001)  (0x0a)[A=0x02][S=nv-BdizC][2]", log_line.to_string());
    }
    
    #[test]
    fn test_adc_set_zero() {
        let cpu_instruction =
        CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x0a]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        registers.accumulator = 0xf6;
        registers.set_c_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 0a)       ADC  #$0a     (#0x1001)  (0x0a)[A=0x00][S=nv-BdiZC][2]", log_line.to_string());
    }

    #[test]
    fn test_adc_set_negative() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0xfa]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xfa]);
        registers.accumulator = 0x01;
        registers.set_c_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xfb, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 fa)       ADC  #$fa     (#0x1001)  (0xfa)[A=0xfb][S=Nv-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_adc_set_overflow() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x50]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x50, 0x02]);
        registers.accumulator = 0x50;
        registers.set_v_flag(false);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0xa0, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(registers.n_flag_is_set());
        assert!(registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 50)       ADC  #$50     (#0x1001)  (0x50)[A=0xa0][S=NV-Bdizc][2]", log_line.to_string());
    }

    #[test]
    fn test_adc_with_overflowing_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0xff]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0xff, 0x02]);
        registers.accumulator = 0x00;
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(2, log_line.cycles); // Immediate mode: 2 cycles
        assert_eq!("#0x1000: (69 ff)       ADC  #$ff     (#0x1001)  (0xff)[A=0x00][S=nv-BdiZC][2]", log_line.to_string());
    }

    #[test]
    fn test_adc_decmode() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x15]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x15, 0x02]);
        registers.accumulator = 0x07;
        registers.set_d_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x22, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles, "ADC in decimal mode should take 3 cycles on 65C02");
        assert_eq!("#0x1000: (69 15)       ADC  #$15     (#0x1001)  (0x15)[A=0x22][S=nv-BDizc][3]", log_line.to_string());
    }

    #[test]
    fn test_adc_decmode_with_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x15]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x15, 0x02]);
        registers.accumulator = 0x07;
        registers.set_c_flag(true);
        registers.set_d_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x23, registers.accumulator);
        assert!(!registers.c_flag_is_set());
        assert!(!registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles, "ADC in decimal mode should take 3 cycles on 65C02");
        assert_eq!("#0x1000: (69 15)       ADC  #$15     (#0x1001)  (0x15)[A=0x23][S=nv-BDizc][3]", log_line.to_string());
    }

    #[test]
    fn test_adc_decmode_giving_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x95]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x95, 0x02]);
        registers.accumulator = 0x05;
        registers.set_d_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles, "ADC in decimal mode should take 3 cycles on 65C02");
        assert_eq!("#0x1000: (69 95)       ADC  #$95     (#0x1001)  (0x95)[A=0x00][S=nv-BDiZC][3]", log_line.to_string());
    }

    #[test]
    fn test_adc_decmode_overflowing_carry() {
        let cpu_instruction =
            CPUInstruction::new(0x1000, 0x69, "ADC", AddressingMode::Immediate([0x99]), adc);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x99, 0x02]);
        registers.accumulator = 0x00;
        registers.set_d_flag(true);
        registers.set_c_flag(true);
        let log_line = cpu_instruction
            .execute(&mut memory, &mut registers)
            .unwrap();
        assert_eq!(0x00, registers.accumulator);
        assert!(registers.c_flag_is_set());
        assert!(registers.z_flag_is_set());
        assert!(!registers.n_flag_is_set());
        assert!(!registers.v_flag_is_set());
        assert_eq!(3, log_line.cycles, "ADC in decimal mode should take 3 cycles on 65C02");
        assert_eq!("#0x1000: (69 99)       ADC  #$99     (#0x1001)  (0x99)[A=0x00][S=nv-BDiZC][3]", log_line.to_string());
    }
}
