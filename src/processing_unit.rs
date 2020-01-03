use super::memory::MemoryStack as Memory;
use super::memory::AddressableIO;
use super::registers::Registers;
use super::addressing_mode::*;
use super::cpu_instruction::{CPUInstruction, LogLine};
use super::cpu_instruction::microcode;
use crate::cpu_instruction::microcode::Result as MicrocodeResult;

fn resolve_opcode(address: usize, opcode: u8, memory: &Memory) -> CPUInstruction {
    let (op1, op2 ) = {
        let y = memory.read(address + 1, 2).unwrap();
        ([y[0]], [y[0], y[1]])
    };
    match opcode {
        0x00    => CPUInstruction::new(address, opcode, "BRK", AddressingMode::Implied, microcode::brk),
        0x1a    => CPUInstruction::new(address, opcode, "INA", AddressingMode::Implied, microcode::ina),
        0x48    => CPUInstruction::new(address, opcode, "PHA", AddressingMode::Implied, microcode::pha),
        0x51    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::eor),
        0x6c    => CPUInstruction::new(address, opcode, "JMP", AddressingMode::Indirect(op2), microcode::jmp),
        0x69    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::Immediate(op1), microcode::adc),
        0x6d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::Absolute(op2), microcode::adc),
        0x7d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::AbsoluteXIndexed(op2), microcode::adc),
        0x8d    => CPUInstruction::new(address, opcode, "STA", AddressingMode::Absolute(op2), microcode::sta),
        0x95    => CPUInstruction::new(address, opcode, "STA", AddressingMode::ZeroPageXIndexed(op1), microcode::sta),
        0x96    => CPUInstruction::new(address, opcode, "STX", AddressingMode::ZeroPageYIndexed(op1), microcode::stx),
        0x9d    => CPUInstruction::new(address, opcode, "STA", AddressingMode::AbsoluteXIndexed(op2), microcode::sta),
        0xa1    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::lda),
        0xa9    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::Immediate(op1), microcode::lda),
        0xaa    => CPUInstruction::new(address, opcode, "TAX", AddressingMode::Implied, microcode::tax),
        0xca    => CPUInstruction::new(address, opcode, "DEX", AddressingMode::Implied, microcode::dex),
        0xd0    => CPUInstruction::new(address, opcode, "BNE", AddressingMode::Relative(op1), microcode::bne),
        0xe8    => CPUInstruction::new(address, opcode, "INX", AddressingMode::Implied, microcode::inx),
        0xed    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::Absolute(op2), microcode::sbc),
        0xf9    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::AbsoluteYIndexed(op2), microcode::sbc),
        _       => panic!("Yet unsupported instruction opcode {:02x} at address #{:04X}.", opcode, address),
    }
}

pub fn execute_step(registers: &mut Registers, memory: &mut Memory) -> MicrocodeResult<LogLine> {
    let cpu_instruction = read_step(registers.command_pointer, memory);
    cpu_instruction.execute(memory, registers)
}

pub fn read_step(address: usize, memory: &Memory) -> CPUInstruction {
    let opcode = memory.read(address, 1).unwrap()[0];
    resolve_opcode(address, opcode, memory)
}

pub fn disassemble(start: usize, end: usize, memory: &Memory) -> Vec<CPUInstruction> {
    let mut cp = start;
    let mut output:Vec<CPUInstruction> = vec![];

    while cp < end {
        let cpu_instruction = read_step(cp, memory);
        cp = cp + 1 + cpu_instruction.addressing_mode.get_operands().len();
        output.push(cpu_instruction);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dex() {
        let mut memory = Memory::new_with_ram();
        let instr:CPUInstruction = resolve_opcode(0x1000, 0xca, &memory);
        assert_eq!("DEX".to_owned(), instr.mnemonic);
        assert_eq!(AddressingMode::Implied, instr.addressing_mode);
    }

    #[test]
    fn test_execute_step_dex() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xca]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x10;

        let _logline:LogLine = execute_step(&mut registers, &mut memory).unwrap();
        assert_eq!(0x0f, registers.register_x);
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn simulate_step_dex() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xca]).unwrap();
        let cpu_instruction:CPUInstruction = read_step(0x1000, &memory);
        assert_eq!(0x1000, cpu_instruction.address);
        assert_eq!("DEX".to_owned(), cpu_instruction.mnemonic);
    }
}
