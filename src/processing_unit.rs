use super::addressing_mode::*;
use super::cpu_instruction::microcode;
use super::cpu_instruction::{CPUInstruction, LogLine};
use super::memory::AddressableIO;
use super::memory::MemoryStack as Memory;
use super::registers::Registers;
use crate::cpu_instruction::microcode::Result as MicrocodeResult;

fn resolve_opcode(address: usize, opcode: u8, memory: &Memory) -> CPUInstruction {
    use microcode as mc;
    use AddressingMode as AM;
    use CPUInstruction as instr;

    let (op1, op2) = {
        let y = memory.read(address + 1, 2).unwrap();
        ([y[0]], [y[0], y[1]])
    };
    match opcode {
        0x00 => instr::new(address, opcode, "BRK", AM::Implied, mc::brk),
        0x01 => instr::new(
            address,
            opcode,
            "ORA",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::ora,
        ),
        0x04 => instr::new(address, opcode, "TSB", AM::ZeroPage(op1), mc::tsb),
        0x05 => instr::new(address, opcode, "ORA", AM::ZeroPage(op1), mc::ora),
        0x06 => instr::new(address, opcode, "ASL", AM::ZeroPage(op1), mc::asl),
        0x08 => instr::new(address, opcode, "PHP", AM::Implied, mc::php),
        0x09 => instr::new(address, opcode, "ORA", AM::Immediate(op1), mc::ora),
        0x0a => instr::new(address, opcode, "ASL", AM::Accumulator, mc::asl),
        0x0c => instr::new(address, opcode, "TSB", AM::Absolute(op2), mc::tsb),
        0x0d => instr::new(address, opcode, "ORA", AM::Absolute(op2), mc::ora),
        0x0e => instr::new(address, opcode, "ASL", AM::Absolute(op2), mc::asl),
        0x10 => instr::new(address, opcode, "BPL", AM::Relative(op1), mc::bpl),
        0x11 => instr::new(
            address,
            opcode,
            "ORA",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::ora,
        ),
        0x12 => instr::new(address, opcode, "ORA", AM::ZeroPageIndirect(op1), mc::ora),
        0x14 => instr::new(address, opcode, "TRB", AM::ZeroPage(op1), mc::trb),
        0x15 => instr::new(address, opcode, "ORA", AM::ZeroPageXIndexed(op1), mc::ora),
        0x16 => instr::new(address, opcode, "ASL", AM::ZeroPageXIndexed(op1), mc::asl),
        0x18 => instr::new(address, opcode, "CLC", AM::Implied, mc::clc),
        0x19 => instr::new(address, opcode, "ORA", AM::AbsoluteYIndexed(op2), mc::ora),
        0x1a => instr::new(address, opcode, "INC", AM::Accumulator, mc::inc),
        0x1c => instr::new(address, opcode, "TRB", AM::Absolute(op2), mc::trb),
        0x1d => instr::new(address, opcode, "ORA", AM::AbsoluteXIndexed(op2), mc::ora),
        0x1e => instr::new(address, opcode, "ASL", AM::AbsoluteXIndexed(op2), mc::asl),
        0x20 => instr::new(address, opcode, "JSR", AM::Absolute(op2), mc::jsr),
        0x21 => instr::new(
            address,
            opcode,
            "AND",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::and,
        ),
        0x24 => instr::new(address, opcode, "BIT", AM::ZeroPage(op1), mc::bit),
        0x25 => instr::new(address, opcode, "AND", AM::ZeroPage(op1), mc::and),
        0x26 => instr::new(address, opcode, "ROL", AM::ZeroPage(op1), mc::rol),
        0x28 => instr::new(address, opcode, "PLP", AM::Implied, mc::plp),
        0x29 => instr::new(address, opcode, "AND", AM::Immediate(op1), mc::and),
        0x2a => instr::new(address, opcode, "ROL", AM::Accumulator, mc::rol),
        0x2c => instr::new(address, opcode, "BIT", AM::Absolute(op2), mc::bit),
        0x2d => instr::new(address, opcode, "AND", AM::Absolute(op2), mc::and),
        0x2e => instr::new(address, opcode, "ROL", AM::Absolute(op2), mc::rol),
        0x30 => instr::new(address, opcode, "BMI", AM::Relative(op1), mc::bmi),
        0x31 => instr::new(
            address,
            opcode,
            "AND",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::and,
        ),
        0x32 => instr::new(address, opcode, "AND", AM::ZeroPageIndirect(op1), mc::and),
        0x34 => instr::new(address, opcode, "BIT", AM::ZeroPageXIndexed(op1), mc::bit),
        0x35 => instr::new(address, opcode, "AND", AM::ZeroPageXIndexed(op1), mc::and),
        0x36 => instr::new(address, opcode, "ROL", AM::ZeroPageXIndexed(op1), mc::rol),
        0x38 => instr::new(address, opcode, "SEC", AM::Implied, mc::sec),
        0x39 => instr::new(address, opcode, "AND", AM::AbsoluteYIndexed(op2), mc::and),
        0x3a => instr::new(address, opcode, "DEC", AM::Accumulator, mc::dec),
        0x3c => instr::new(address, opcode, "BIT", AM::AbsoluteXIndexed(op2), mc::bit),
        0x3d => instr::new(address, opcode, "AND", AM::AbsoluteXIndexed(op2), mc::and),
        0x3e => instr::new(address, opcode, "ROL", AM::AbsoluteXIndexed(op2), mc::rol),
        0x40 => instr::new(address, opcode, "RTI", AM::Implied, mc::rti),
        0x41 => instr::new(
            address,
            opcode,
            "EOR",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::eor,
        ),
        0x45 => instr::new(address, opcode, "EOR", AM::ZeroPage(op1), mc::eor),
        0x46 => instr::new(address, opcode, "LSR", AM::ZeroPage(op1), mc::lsr),
        0x48 => instr::new(address, opcode, "PHA", AM::Implied, mc::pha),
        0x49 => instr::new(address, opcode, "EOR", AM::Immediate(op1), mc::eor),
        0x4a => instr::new(address, opcode, "LSR", AM::Accumulator, mc::lsr),
        0x4c => instr::new(address, opcode, "JMP", AM::Absolute(op2), mc::jmp),
        0x4d => instr::new(address, opcode, "EOR", AM::Absolute(op2), mc::eor),
        0x4e => instr::new(address, opcode, "LSR", AM::Absolute(op2), mc::lsr),
        0x50 => instr::new(address, opcode, "BVC", AM::Relative(op1), mc::bvc),
        0x51 => instr::new(
            address,
            opcode,
            "EOR",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::eor,
        ),
        0x52 => instr::new(address, opcode, "EOR", AM::ZeroPageIndirect(op1), mc::eor),
        0x55 => instr::new(address, opcode, "EOR", AM::ZeroPageXIndexed(op1), mc::eor),
        0x56 => instr::new(address, opcode, "LSR", AM::ZeroPageXIndexed(op1), mc::lsr),
        0x58 => instr::new(address, opcode, "CLI", AM::Implied, mc::cli),
        0x59 => instr::new(address, opcode, "EOR", AM::AbsoluteYIndexed(op2), mc::eor),
        0x5a => instr::new(address, opcode, "PHY", AM::Implied, mc::phy),
        0x5d => instr::new(address, opcode, "EOR", AM::AbsoluteXIndexed(op2), mc::eor),
        0x5e => instr::new(address, opcode, "LSR", AM::AbsoluteXIndexed(op2), mc::lsr),
        0x60 => instr::new(address, opcode, "RTS", AM::Implied, mc::rts),
        0x61 => instr::new(
            address,
            opcode,
            "ADC",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::adc,
        ),
        0x64 => instr::new(address, opcode, "STZ", AM::ZeroPage(op1), mc::stz),
        0x65 => instr::new(address, opcode, "ADC", AM::ZeroPage(op1), mc::adc),
        0x66 => instr::new(address, opcode, "ROR", AM::ZeroPage(op1), mc::ror),
        0x68 => instr::new(address, opcode, "PLA", AM::Implied, mc::pla),
        0x69 => instr::new(address, opcode, "ADC", AM::Immediate(op1), mc::adc),
        0x6a => instr::new(address, opcode, "ROR", AM::Accumulator, mc::ror),
        0x6c => instr::new(address, opcode, "JMP", AM::Indirect(op2), mc::jmp),
        0x6d => instr::new(address, opcode, "ADC", AM::Absolute(op2), mc::adc),
        0x6e => instr::new(address, opcode, "ROR", AM::Absolute(op2), mc::ror),
        0x70 => instr::new(address, opcode, "BVS", AM::Relative(op1), mc::bvs),
        0x71 => instr::new(
            address,
            opcode,
            "ADC",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::adc,
        ),
        0x72 => instr::new(address, opcode, "ADC", AM::ZeroPageIndirect(op1), mc::adc),
        0x74 => instr::new(address, opcode, "STZ", AM::ZeroPageXIndexed(op1), mc::stz),
        0x75 => instr::new(address, opcode, "ADC", AM::ZeroPageXIndexed(op1), mc::adc),
        0x76 => instr::new(address, opcode, "ROR", AM::ZeroPageXIndexed(op1), mc::ror),
        0x78 => instr::new(address, opcode, "SEI", AM::Implied, mc::sei),
        0x79 => instr::new(address, opcode, "ADC", AM::AbsoluteYIndexed(op2), mc::adc),
        0x7a => instr::new(address, opcode, "PLY", AM::Implied, mc::ply),
        0x7c => instr::new(
            address,
            opcode,
            "JMP",
            AM::AbsoluteXIndexedIndirect(op2),
            mc::jmp,
        ),
        0x7d => instr::new(address, opcode, "ADC", AM::AbsoluteXIndexed(op2), mc::adc),
        0x7e => instr::new(address, opcode, "ROR", AM::AbsoluteXIndexed(op2), mc::ror),
        0x80 => instr::new(address, opcode, "BRA", AM::Relative(op1), mc::bra),
        0x81 => instr::new(
            address,
            opcode,
            "STA",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::sta,
        ),
        0x84 => instr::new(address, opcode, "STY", AM::ZeroPage(op1), mc::sty),
        0x85 => instr::new(address, opcode, "STA", AM::ZeroPage(op1), mc::sta),
        0x86 => instr::new(address, opcode, "STX", AM::ZeroPage(op1), mc::stx),
        0x88 => instr::new(address, opcode, "DEY", AM::Implied, mc::dey),
        0x89 => instr::new(address, opcode, "BIT", AM::Immediate(op1), mc::bit),
        0x8a => instr::new(address, opcode, "TXA", AM::Implied, mc::txa),
        0x8c => instr::new(address, opcode, "STY", AM::Absolute(op2), mc::sty),
        0x8d => instr::new(address, opcode, "STA", AM::Absolute(op2), mc::sta),
        0x8e => instr::new(address, opcode, "STX", AM::Absolute(op2), mc::stx),
        0x90 => instr::new(address, opcode, "BCC", AM::Relative(op1), mc::bcc),
        0x91 => instr::new(
            address,
            opcode,
            "STA",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::sta,
        ),
        0x92 => instr::new(address, opcode, "STA", AM::ZeroPageIndirect(op1), mc::sta),
        0x94 => instr::new(address, opcode, "STY", AM::ZeroPageXIndexed(op1), mc::sty),
        0x95 => instr::new(address, opcode, "STA", AM::ZeroPageXIndexed(op1), mc::sta),
        0x96 => instr::new(address, opcode, "STX", AM::ZeroPageYIndexed(op1), mc::stx),
        0x98 => instr::new(address, opcode, "TYA", AM::Implied, mc::tya),
        0x99 => instr::new(address, opcode, "STA", AM::AbsoluteYIndexed(op2), mc::sta),
        0x9a => instr::new(address, opcode, "TXS", AM::Implied, mc::txs),
        0x9c => instr::new(address, opcode, "STZ", AM::Absolute(op2), mc::stz),
        0x9d => instr::new(address, opcode, "STA", AM::AbsoluteXIndexed(op2), mc::sta),
        0x9e => instr::new(address, opcode, "STZ", AM::AbsoluteXIndexed(op2), mc::stz),
        0xa0 => instr::new(address, opcode, "LDY", AM::Immediate(op1), mc::ldy),
        0xa1 => instr::new(
            address,
            opcode,
            "LDA",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::lda,
        ),
        0xa2 => instr::new(address, opcode, "LDX", AM::Immediate(op1), mc::ldx),
        0xa4 => instr::new(address, opcode, "LDY", AM::ZeroPage(op1), mc::ldy),
        0xa5 => instr::new(address, opcode, "LDA", AM::ZeroPage(op1), mc::lda),
        0xa6 => instr::new(address, opcode, "LDX", AM::ZeroPage(op1), mc::ldx),
        0xa8 => instr::new(address, opcode, "TAY", AM::Implied, mc::tay),
        0xa9 => instr::new(address, opcode, "LDA", AM::Immediate(op1), mc::lda),
        0xaa => instr::new(address, opcode, "TAX", AM::Implied, mc::tax),
        0xac => instr::new(address, opcode, "LDY", AM::Absolute(op2), mc::ldy),
        0xad => instr::new(address, opcode, "LDA", AM::Absolute(op2), mc::lda),
        0xae => instr::new(address, opcode, "LDX", AM::Absolute(op2), mc::ldx),
        0xb0 => instr::new(address, opcode, "BCS", AM::Relative(op1), mc::bcs),
        0xb1 => instr::new(
            address,
            opcode,
            "LDA",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::lda,
        ),
        0xb2 => instr::new(address, opcode, "LDA", AM::ZeroPageIndirect(op1), mc::lda),
        0xb4 => instr::new(address, opcode, "LDY", AM::ZeroPageXIndexed(op1), mc::ldy),
        0xb5 => instr::new(address, opcode, "LDA", AM::ZeroPageXIndexed(op1), mc::lda),
        0xb6 => instr::new(address, opcode, "LDX", AM::ZeroPageYIndexed(op1), mc::ldx),
        0xb8 => instr::new(address, opcode, "CLV", AM::Implied, mc::clv),
        0xb9 => instr::new(address, opcode, "LDA", AM::AbsoluteYIndexed(op2), mc::lda),
        0xba => instr::new(address, opcode, "TSX", AM::Implied, mc::tsx),
        0xbc => instr::new(address, opcode, "LDY", AM::AbsoluteXIndexed(op2), mc::ldy),
        0xbd => instr::new(address, opcode, "LDA", AM::AbsoluteXIndexed(op2), mc::lda),
        0xbe => instr::new(address, opcode, "LDX", AM::AbsoluteYIndexed(op2), mc::ldx),
        0xc0 => instr::new(address, opcode, "CPY", AM::Immediate(op1), mc::cpy),
        0xc1 => instr::new(
            address,
            opcode,
            "CMP",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::cmp,
        ),
        0xc4 => instr::new(address, opcode, "CPY", AM::ZeroPage(op1), mc::cpy),
        0xc5 => instr::new(address, opcode, "CMP", AM::ZeroPage(op1), mc::cmp),
        0xc6 => instr::new(address, opcode, "DEC", AM::ZeroPage(op1), mc::dec),
        0xc8 => instr::new(address, opcode, "INY", AM::Implied, mc::iny),
        0xc9 => instr::new(address, opcode, "CMP", AM::Immediate(op1), mc::cmp),
        0xca => instr::new(address, opcode, "DEX", AM::Implied, mc::dex),
        0xcc => instr::new(address, opcode, "CPY", AM::Absolute(op2), mc::cpy),
        0xcd => instr::new(address, opcode, "CMP", AM::Absolute(op2), mc::cmp),
        0xce => instr::new(address, opcode, "DEC", AM::Absolute(op2), mc::dec),
        0xd0 => instr::new(address, opcode, "BNE", AM::Relative(op1), mc::bne),
        0xd1 => instr::new(
            address,
            opcode,
            "CMP",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::cmp,
        ),
        0xd2 => instr::new(address, opcode, "CMP", AM::ZeroPageIndirect(op1), mc::cmp),
        0xd6 => instr::new(address, opcode, "DEC", AM::ZeroPageXIndexed(op1), mc::dec),
        0xdb => instr::new(address, opcode, "STP", AM::Implied, mc::stp),
        0xd5 => instr::new(address, opcode, "CMP", AM::ZeroPageXIndexed(op1), mc::cmp),
        0xd8 => instr::new(address, opcode, "CLD", AM::Implied, mc::cld),
        0xd9 => instr::new(address, opcode, "CMP", AM::AbsoluteYIndexed(op2), mc::cmp),
        0xda => instr::new(address, opcode, "PHX", AM::Implied, mc::phx),
        0xdd => instr::new(address, opcode, "CMP", AM::AbsoluteXIndexed(op2), mc::cmp),
        0xde => instr::new(address, opcode, "DEC", AM::AbsoluteXIndexed(op2), mc::dec),
        0xe0 => instr::new(address, opcode, "CPX", AM::Immediate(op1), mc::cpx),
        0xe1 => instr::new(
            address,
            opcode,
            "SBC",
            AM::ZeroPageXIndexedIndirect(op1),
            mc::sbc,
        ),
        0xe4 => instr::new(address, opcode, "CPX", AM::ZeroPage(op1), mc::cpx),
        0xe5 => instr::new(address, opcode, "SBC", AM::ZeroPage(op1), mc::sbc),
        0xe6 => instr::new(address, opcode, "INC", AM::ZeroPage(op1), mc::inc),
        0xe8 => instr::new(address, opcode, "INX", AM::Implied, mc::inx),
        0xe9 => instr::new(address, opcode, "SBC", AM::Immediate(op1), mc::sbc),
        0xea => instr::new(address, opcode, "NOP", AM::Implied, mc::nop),
        0xec => instr::new(address, opcode, "CPX", AM::Absolute(op2), mc::cpx),
        0xed => instr::new(address, opcode, "SBC", AM::Absolute(op2), mc::sbc),
        0xee => instr::new(address, opcode, "INC", AM::Absolute(op2), mc::inc),
        0xf0 => instr::new(address, opcode, "BEQ", AM::Relative(op1), mc::beq),
        0xf1 => instr::new(
            address,
            opcode,
            "SBC",
            AM::ZeroPageIndirectYIndexed(op1),
            mc::sbc,
        ),
        0xf2 => instr::new(address, opcode, "SBC", AM::ZeroPageIndirect(op1), mc::sbc),
        0xf5 => instr::new(address, opcode, "SBC", AM::ZeroPageXIndexed(op1), mc::sbc),
        0xf6 => instr::new(address, opcode, "INC", AM::ZeroPageXIndexed(op1), mc::inc),
        0xf8 => instr::new(address, opcode, "SED", AM::Implied, mc::sed),
        0xf9 => instr::new(address, opcode, "SBC", AM::AbsoluteYIndexed(op2), mc::sbc),
        0xfa => instr::new(address, opcode, "PLX", AM::Implied, mc::plx),
        0xfd => instr::new(address, opcode, "SBC", AM::AbsoluteXIndexed(op2), mc::sbc),
        0xfe => instr::new(address, opcode, "INC", AM::AbsoluteXIndexed(op2), mc::inc),
        _ => panic!(
            "Yet unsupported instruction opcode 0x{:02x} at address #0x{:04X}.",
            opcode, address
        ),
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
    let mut output: Vec<CPUInstruction> = vec![];

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
        let instr: CPUInstruction = resolve_opcode(0x1000, 0xca, &memory);
        assert_eq!("DEX".to_owned(), instr.mnemonic);
        assert_eq!(AddressingMode::Implied, instr.addressing_mode);
    }

    #[test]
    fn test_execute_step_dex() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xca]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x10;

        let _logline: LogLine = execute_step(&mut registers, &mut memory).unwrap();
        assert_eq!(0x0f, registers.register_x);
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn simulate_step_dex() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xca]).unwrap();
        let cpu_instruction: CPUInstruction = read_step(0x1000, &memory);
        assert_eq!(0x1000, cpu_instruction.address);
        assert_eq!("DEX".to_owned(), cpu_instruction.mnemonic);
    }
}
