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
        0x06    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::ZeroPage(op1), microcode::asl),
        0x08    => CPUInstruction::new(address, opcode, "PLA", AddressingMode::Implied, microcode::pla),
        0x0a    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::Accumulator, microcode::asl),
        0x0e    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::Absolute(op2), microcode::asl),
        0x16    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::ZeroPageXIndexed(op1), microcode::asl),
        0x1a    => CPUInstruction::new(address, opcode, "INC", AddressingMode::Accumulator, microcode::inc),
        0x1e    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::AbsoluteXIndexed(op2), microcode::asl),
        0x21    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::and),
        0x25    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPage(op1), microcode::and),
        0x29    => CPUInstruction::new(address, opcode, "AND", AddressingMode::Immediate(op1), microcode::and),
        0x2d    => CPUInstruction::new(address, opcode, "AND", AddressingMode::Absolute(op2), microcode::and),
        0x31    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::and),
        0x32    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageIndirect(op1), microcode::and),
        0x35    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageXIndexed(op1), microcode::and),
        0x39    => CPUInstruction::new(address, opcode, "AND", AddressingMode::AbsoluteYIndexed(op2), microcode::and),
        0x3d    => CPUInstruction::new(address, opcode, "AND", AddressingMode::AbsoluteXIndexed(op2), microcode::and),
        0x48    => CPUInstruction::new(address, opcode, "PHA", AddressingMode::Implied, microcode::pha),
        0x51    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::eor),
        0x61    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::adc),
        0x65    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPage(op1), microcode::adc),
        0x69    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::Immediate(op1), microcode::adc),
        0x6c    => CPUInstruction::new(address, opcode, "JMP", AddressingMode::Indirect(op2), microcode::jmp),
        0x6d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::Absolute(op2), microcode::adc),
        0x71    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::adc),
        0x72    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageIndirect(op1), microcode::adc),
        0x75    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageXIndexed(op1), microcode::adc),
        0x79    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::AbsoluteYIndexed(op2), microcode::adc),
        0x7d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::AbsoluteXIndexed(op2), microcode::adc),
        0x80    => CPUInstruction::new(address, opcode, "BRA", AddressingMode::Relative(op1), microcode::bra),
        0x85    => CPUInstruction::new(address, opcode, "STA", AddressingMode::ZeroPage(op1), microcode::sta),
        0x88    => CPUInstruction::new(address, opcode, "DEY", AddressingMode::Implied, microcode::dey),
        0x8d    => CPUInstruction::new(address, opcode, "STA", AddressingMode::Absolute(op2), microcode::sta),
        0x91    => CPUInstruction::new(address, opcode, "STA", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::sta),
        0x95    => CPUInstruction::new(address, opcode, "STA", AddressingMode::ZeroPageXIndexed(op1), microcode::sta),
        0x96    => CPUInstruction::new(address, opcode, "STX", AddressingMode::ZeroPageYIndexed(op1), microcode::stx),
        0x99    => CPUInstruction::new(address, opcode, "STA", AddressingMode::AbsoluteYIndexed(op2), microcode::sta),
        0x9c    => CPUInstruction::new(address, opcode, "STZ", AddressingMode::Absolute(op2), microcode::stz),
        0x9d    => CPUInstruction::new(address, opcode, "STA", AddressingMode::AbsoluteXIndexed(op2), microcode::sta),
        0xa0    => CPUInstruction::new(address, opcode, "LDY", AddressingMode::Immediate(op1), microcode::ldy),
        0xa1    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::lda),
        0xa2    => CPUInstruction::new(address, opcode, "LDX", AddressingMode::Immediate(op1), microcode::ldx),
        0xa4    => CPUInstruction::new(address, opcode, "LDY", AddressingMode::ZeroPage(op1), microcode::ldy),
        0xa9    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::Immediate(op1), microcode::lda),
        0xaa    => CPUInstruction::new(address, opcode, "TAX", AddressingMode::Implied, microcode::tax),
        0xad    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::Absolute(op2), microcode::lda),
        0xc8    => CPUInstruction::new(address, opcode, "INY", AddressingMode::Implied, microcode::iny),
        0xca    => CPUInstruction::new(address, opcode, "DEX", AddressingMode::Implied, microcode::dex),
        0xd0    => CPUInstruction::new(address, opcode, "BNE", AddressingMode::Relative(op1), microcode::bne),
        0xe5    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::ZeroPage(op1), microcode::sbc),
        0xe8    => CPUInstruction::new(address, opcode, "INX", AddressingMode::Implied, microcode::inx),
        0xea    => CPUInstruction::new(address, opcode, "NOP", AddressingMode::Implied, microcode::nop),
        0xed    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::Absolute(op2), microcode::sbc),
        0xf0    => CPUInstruction::new(address, opcode, "BEQ", AddressingMode::Relative(op1), microcode::beq),
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
