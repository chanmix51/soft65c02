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
        0x10    => CPUInstruction::new(address, opcode, "BPL", AddressingMode::Relative(op1), microcode::bpl),
        0x16    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::ZeroPageXIndexed(op1), microcode::asl),
        0x18    => CPUInstruction::new(address, opcode, "CLC", AddressingMode::Implied, microcode::clc),
        0x1a    => CPUInstruction::new(address, opcode, "INC", AddressingMode::Accumulator, microcode::inc),
        0x1e    => CPUInstruction::new(address, opcode, "ASL", AddressingMode::AbsoluteXIndexed(op2), microcode::asl),
        0x20    => CPUInstruction::new(address, opcode, "JSR", AddressingMode::Absolute(op2), microcode::jsr),
        0x21    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::and),
        0x24    => CPUInstruction::new(address, opcode, "BIT", AddressingMode::ZeroPage(op1), microcode::bit),
        0x25    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPage(op1), microcode::and),
        0x29    => CPUInstruction::new(address, opcode, "AND", AddressingMode::Immediate(op1), microcode::and),
        0x2c    => CPUInstruction::new(address, opcode, "BIT", AddressingMode::Absolute(op2), microcode::bit),
        0x2d    => CPUInstruction::new(address, opcode, "AND", AddressingMode::Absolute(op2), microcode::and),
        0x30    => CPUInstruction::new(address, opcode, "BMI", AddressingMode::Relative(op1), microcode::bmi),
        0x31    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::and),
        0x32    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageIndirect(op1), microcode::and),
        0x34    => CPUInstruction::new(address, opcode, "BIT", AddressingMode::ZeroPageXIndexed(op1), microcode::bit),
        0x35    => CPUInstruction::new(address, opcode, "AND", AddressingMode::ZeroPageXIndexed(op1), microcode::and),
        0x39    => CPUInstruction::new(address, opcode, "AND", AddressingMode::AbsoluteYIndexed(op2), microcode::and),
        0x3a    => CPUInstruction::new(address, opcode, "DEC", AddressingMode::Accumulator, microcode::dec),
        0x3c    => CPUInstruction::new(address, opcode, "BIT", AddressingMode::AbsoluteXIndexed(op2), microcode::bit),
        0x3d    => CPUInstruction::new(address, opcode, "AND", AddressingMode::AbsoluteXIndexed(op2), microcode::and),
        0x41    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::eor),
        0x45    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPage(op1), microcode::eor),
        0x48    => CPUInstruction::new(address, opcode, "PHA", AddressingMode::Implied, microcode::pha),
        0x49    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::Immediate(op1), microcode::eor),
        0x4c    => CPUInstruction::new(address, opcode, "JMP", AddressingMode::Absolute(op2), microcode::jmp),
        0x4d    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::Absolute(op2), microcode::eor),
        0x50    => CPUInstruction::new(address, opcode, "BVC", AddressingMode::Relative(op1), microcode::bvc),
        0x51    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::eor),
        0x52    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageIndirect(op1), microcode::eor),
        0x55    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::ZeroPageXIndexed(op1), microcode::eor),
        0x58    => CPUInstruction::new(address, opcode, "CLI", AddressingMode::Implied, microcode::cli),
        0x59    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::AbsoluteYIndexed(op2), microcode::eor),
        0x5d    => CPUInstruction::new(address, opcode, "EOR", AddressingMode::AbsoluteXIndexed(op2), microcode::eor),
        0x61    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::adc),
        0x65    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPage(op1), microcode::adc),
        0x69    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::Immediate(op1), microcode::adc),
        0x6c    => CPUInstruction::new(address, opcode, "JMP", AddressingMode::Indirect(op2), microcode::jmp),
        0x6d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::Absolute(op2), microcode::adc),
        0x70    => CPUInstruction::new(address, opcode, "BVS", AddressingMode::Relative(op1), microcode::bvs),
        0x71    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::adc),
        0x72    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageIndirect(op1), microcode::adc),
        0x75    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::ZeroPageXIndexed(op1), microcode::adc),
        0x79    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::AbsoluteYIndexed(op2), microcode::adc),
        0x7c    => CPUInstruction::new(address, opcode, "JMP", AddressingMode::AbsoluteXIndexedIndirect(op2), microcode::jmp),
        0x7d    => CPUInstruction::new(address, opcode, "ADC", AddressingMode::AbsoluteXIndexed(op2), microcode::adc),
        0x80    => CPUInstruction::new(address, opcode, "BRA", AddressingMode::Relative(op1), microcode::bra),
        0x85    => CPUInstruction::new(address, opcode, "STA", AddressingMode::ZeroPage(op1), microcode::sta),
        0x88    => CPUInstruction::new(address, opcode, "DEY", AddressingMode::Implied, microcode::dey),
        0x89    => CPUInstruction::new(address, opcode, "BIT", AddressingMode::Immediate(op1), microcode::bit),
        0x8d    => CPUInstruction::new(address, opcode, "STA", AddressingMode::Absolute(op2), microcode::sta),
        0x90    => CPUInstruction::new(address, opcode, "BCC", AddressingMode::Relative(op1), microcode::bcc),
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
        0xa5    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPage(op1), microcode::lda),
        0xa9    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::Immediate(op1), microcode::lda),
        0xaa    => CPUInstruction::new(address, opcode, "TAX", AddressingMode::Implied, microcode::tax),
        0xad    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::Absolute(op2), microcode::lda),
        0xb0    => CPUInstruction::new(address, opcode, "BCS", AddressingMode::Relative(op1), microcode::bcs),
        0xb1    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::lda),
        0xb2    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPageIndirect(op1), microcode::lda),
        0xb5    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::ZeroPageXIndexed(op1), microcode::lda),
        0xb8    => CPUInstruction::new(address, opcode, "CLV", AddressingMode::Implied, microcode::clv),
        0xb9    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::AbsoluteYIndexed(op2), microcode::lda),
        0xbd    => CPUInstruction::new(address, opcode, "LDA", AddressingMode::AbsoluteXIndexed(op2), microcode::lda),
        0xc0    => CPUInstruction::new(address, opcode, "CPY", AddressingMode::Immediate(op1), microcode::cpy),
        0xc1    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::ZeroPageXIndexedIndirect(op1), microcode::cmp),
        0xc4    => CPUInstruction::new(address, opcode, "CPY", AddressingMode::ZeroPage(op1), microcode::cpy),
        0xc5    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::ZeroPage(op1), microcode::cmp),
        0xc6    => CPUInstruction::new(address, opcode, "DEC", AddressingMode::ZeroPage(op1), microcode::dec),
        0xc8    => CPUInstruction::new(address, opcode, "INY", AddressingMode::Implied, microcode::iny),
        0xc9    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::Immediate(op1), microcode::cmp),
        0xca    => CPUInstruction::new(address, opcode, "DEX", AddressingMode::Implied, microcode::dex),
        0xcc    => CPUInstruction::new(address, opcode, "CPY", AddressingMode::Absolute(op2), microcode::cpy),
        0xcd    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::Absolute(op2), microcode::cmp),
        0xce    => CPUInstruction::new(address, opcode, "DEC", AddressingMode::Absolute(op2), microcode::dec),
        0xd0    => CPUInstruction::new(address, opcode, "BNE", AddressingMode::Relative(op1), microcode::bne),
        0xd1    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::ZeroPageIndirectYIndexed(op1), microcode::cmp),
        0xd2    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::ZeroPageIndirect(op1), microcode::cmp),
        0xd6    => CPUInstruction::new(address, opcode, "DEC", AddressingMode::ZeroPageXIndexed(op1), microcode::dec),
        0xdb    => CPUInstruction::new(address, opcode, "STP", AddressingMode::Implied, microcode::stp),
        0xd5    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::ZeroPageXIndexed(op1), microcode::cmp),
        0xd8    => CPUInstruction::new(address, opcode, "CLD", AddressingMode::Implied, microcode::cld),
        0xd9    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::AbsoluteYIndexed(op2), microcode::cmp),
        0xdd    => CPUInstruction::new(address, opcode, "CMP", AddressingMode::AbsoluteXIndexed(op2), microcode::cmp),
        0xde    => CPUInstruction::new(address, opcode, "DEC", AddressingMode::AbsoluteXIndexed(op2), microcode::dec),
        0xe0    => CPUInstruction::new(address, opcode, "CPX", AddressingMode::Immediate(op1), microcode::cpx),
        0xe4    => CPUInstruction::new(address, opcode, "CPX", AddressingMode::ZeroPage(op1), microcode::cpx),
        0xe5    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::ZeroPage(op1), microcode::sbc),
        0xe6    => CPUInstruction::new(address, opcode, "INC", AddressingMode::ZeroPage(op1), microcode::inc),
        0xe8    => CPUInstruction::new(address, opcode, "INX", AddressingMode::Implied, microcode::inx),
        0xea    => CPUInstruction::new(address, opcode, "NOP", AddressingMode::Implied, microcode::nop),
        0xec    => CPUInstruction::new(address, opcode, "CPX", AddressingMode::Absolute(op2), microcode::cpx),
        0xed    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::Absolute(op2), microcode::sbc),
        0xee    => CPUInstruction::new(address, opcode, "INC", AddressingMode::Absolute(op2), microcode::inc),
        0xf0    => CPUInstruction::new(address, opcode, "BEQ", AddressingMode::Relative(op1), microcode::beq),
        0xf6    => CPUInstruction::new(address, opcode, "INC", AddressingMode::ZeroPageXIndexed(op1), microcode::inc),
        0xf9    => CPUInstruction::new(address, opcode, "SBC", AddressingMode::AbsoluteYIndexed(op2), microcode::sbc),
        0xfe    => CPUInstruction::new(address, opcode, "INC", AddressingMode::AbsoluteXIndexed(op2), microcode::inc),
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
