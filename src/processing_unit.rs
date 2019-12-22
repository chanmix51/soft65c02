use super::memory::RAM as Memory;
use super::registers::Registers;
use super::addressing_mode::*;
use super::cpu_instruction::{CPUInstruction, LogLine};
use super::cpu_instruction::microcode;

fn resolve_opcode(address: usize, opcode: u8) -> CPUInstruction {
    match opcode {
        0x00    => CPUInstruction::new(address, opcode, "BRK", AddressingMode::Implied, microcode::brk),
        0x48    => CPUInstruction::new(address, opcode, "PHA", AddressingMode::Implied, microcode::pha),
        0x51    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageIndirectYIndexed, microcode::eor),
        0x6c    => CPUInstruction::new(address, opcode, "JMP", AddressingMode::Indirect, microcode::jmp),
        0x7d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::AbsoluteXIndexed, microcode::adc),
        0x8d    => CPUInstruction::new(address, opcode, "STA", AddressingMode::Absolute, microcode::sta),
        0x95    => CPUInstruction::new(address, opcode, "STA", AddressingMode::ZeroPageXIndexed, microcode::sta),
        0x96    => CPUInstruction::new(address, opcode, "STX", AddressingMode::ZeroPageYIndexed, microcode::stx),
        0xa1    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPageXIndexedIndirect, microcode::lda),
        0xa9    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::Immediate, microcode::lda),
        0xca    => CPUInstruction::new(address, opcode, "DEX", AddressingMode::Implied, microcode::dex),
        0xd0    => CPUInstruction::new(address, opcode, "BNE", AddressingMode::Relative, microcode::bne),
        0xf9    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::AbsoluteYIndexed, microcode::sbc),
        _       => panic!("Yet unsupported instruction opcode {:02x} at address #{:04X}.", opcode, address),
    }
}

fn execute_step(registers: &mut Registers, memory: &mut Memory) -> LogLine {
    let opcode = memory.read(registers.command_pointer, 1).unwrap()[0];
    let cpu_instruction = resolve_opcode(registers.command_pointer, opcode);
    cpu_instruction.execute(memory, registers)
}

fn read_step(address: usize, registers: &Registers, memory: &Memory) -> LogLine {
    let opcode = memory.read(address, 1).unwrap()[0];
    let cpu_instruction = resolve_opcode(address, opcode);
    cpu_instruction.simulate(memory, registers)
}

pub fn disassemble(start: usize, end: usize, registers: &Registers, memory: &Memory) -> Vec<LogLine> {
    let mut cp = start;
    let mut output:Vec<LogLine> = vec![];

    while cp < end {
        let log_line = read_step(cp, registers, memory);
        println!("{}", log_line);
        cp = cp + 1 + log_line.resolution.operands.len();
        output.push(log_line);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex() {
        let instr:CPUInstruction = resolve_opcode(0x1000, 0xca);
        assert_eq!("DEX".to_owned(), instr.mnemonic);
        assert_eq!(AddressingMode::Implied, instr.addressing_mode);
    }

    #[test]
    fn test_execute_step_dex() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xca]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x10;

        let _logline:LogLine = execute_step(&mut registers, &mut memory);
        assert_eq!(0x0f, registers.register_x);
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn simulate_step_dex() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xca]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x10;
        let logline:LogLine = read_step(0x1000, &registers, &memory);
        assert_eq!(0x1000, logline.address);
        assert_eq!("DEX".to_owned(), logline.mnemonic);
    }
}
