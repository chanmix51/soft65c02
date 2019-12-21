use super::memory::RAM as Memory;
use super::registers::Registers;
use super::addressing_mode::*;
use super::cpu_instruction::{CPUInstruction, LogLine};
use super::cpu_instruction::microcode;

fn resolve_opcode(address: usize, opcode: u8) -> CPUInstruction {
    match opcode {
        0xca    => CPUInstruction::new(address, opcode, "DEX", AddressingMode::Implied, microcode::dex),
        _       => panic!("Yet unsupported instruction opcode {:02x} at address #{:04X}.", opcode, address),
    }
}

fn execute_step(registers: &mut Registers, memory: &mut Memory) -> LogLine {
    let opcode = memory.read(registers.command_pointer, 1).unwrap()[0];
    let cpu_instruction = resolve_opcode(registers.command_pointer, opcode);
    cpu_instruction.execute(memory, registers)
}

fn read_step(address: usize, registers: &Registers, memory: &Memory) -> LogLine {
    let opcode = memory.read(registers.command_pointer, 1).unwrap()[0];
    let cpu_instruction = resolve_opcode(registers.command_pointer, opcode);
    cpu_instruction.simulate(memory, registers)
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
        memory.write(0x1000, vec![0xca]);
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x10;

        let logline:LogLine = execute_step(&mut registers, &mut memory);
        assert_eq!(0x0f, registers.register_x);
        assert_eq!(0x1001, registers.command_pointer);
    }

    #[test]
    fn simulate_step_dex() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xca]);
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x10;
        let logline:LogLine = read_step(0x1000, &registers, &memory);
        assert_eq!(0x1000, logline.address);
        assert_eq!("DEX".to_owned(), logline.mnemonic);
    }
}
