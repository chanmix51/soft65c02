use crate::cpu_instruction::{CPUInstruction, LogLine};
use crate::registers::Registers;
use crate::memory::RAM as Memory;
use crate::addressing_mode::*;

pub fn bne(memory: &mut Memory, registers: &mut Registers, cpu_instruction: &CPUInstruction) -> LogLine {
    let resolution = cpu_instruction.addressing_mode.solve(registers.command_pointer, memory, registers);
    let target_address = match resolution.target_address {
        Some(v) => v,
        None => panic!("Ooops no target address from the addressing mode resolver."),
    };

    if registers.status_register & 0b01000000 != 0 {
        registers.command_pointer += 1 + { let a = resolution.operands[0] as usize; if a > 127 { a - 256 } else { a }};
    } else {
        registers.command_pointer += 1 + resolution.operands.len();
    }

    LogLine {
        address:    cpu_instruction.address,
        opcode:     cpu_instruction.opcode,
        mnemonic:   cpu_instruction.mnemonic.clone(),
        resolution: resolution,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu_instruction::cpu_instruction::tests::get_stuff;

    #[test]
    fn test_bne() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "bne", AddressingMode::Immediate, bne);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0x4c, 0x0a, 0x02]);
        let log_line = cpu_instruction.execute(&mut memory, &mut registers);
        assert_eq!("bne".to_owned(), log_line.mnemonic);
        assert_eq!(0x100a, registers.command_pointer);
    }
}
