use super::memory::RAM as Memory;
use super::registers::Registers;
use super::addressing_mode::*;

struct CPUInstruction {
    address:    usize,
    opcode:     u8,
    mnemonic:   String,
    addressing_mode: AddressingMode,
    microcode:  Box<dyn Fn(&mut Memory, &mut Registers, &AddressingMode) -> String>,
}

impl CPUInstruction {
    pub fn new(address: usize, opcode: u8, mnemonic: &str, addressing_mode: AddressingMode, microcode: impl Fn(&mut Memory, &mut Registers, &AddressingMode) -> String + 'static) -> CPUInstruction {
        CPUInstruction {
            address:            address,
            opcode:             opcode,
            mnemonic:           mnemonic.to_owned(),
            addressing_mode:    addressing_mode,
            microcode:          Box::new(microcode)
        }
    }

    pub fn execute(&self, memory: &mut Memory, registers: &mut Registers) -> String {
        (self.microcode)(memory, registers, &self.addressing_mode)
    }
}

pub fn DEX(memory: &mut Memory, registers: &mut Registers, addressing_mode: &AddressingMode) -> String {
    let am_resolution = addressing_mode.solve(registers.command_pointer, memory, registers);

    if registers.register_x != 0 {
        registers.register_x -= 1;
        if registers.register_x == 0 {
            registers.status_register |= 0b00000010;
        } else {
            registers.status_register &= 0b01111101;
        }
    } else {
        registers.register_x = 0xff;
        registers.status_register |= 0b10000000;
    }

    registers.command_pointer += 1 + am_resolution.operands.len();

    "Pika".to_owned()

}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_stuff(addr: usize, program: Vec<u8>) -> (Memory, Registers) {
        let mut memory = Memory::new();
        memory.write(addr, program);
        let mut registers = Registers::new(addr);

        (memory, registers)
    }

    #[test]
    fn test_dex() {
        let cpu_instruction = CPUInstruction::new(0x1000, 0xca, "DEX", AddressingMode::Implied, DEX);
        let (mut memory, mut registers) = get_stuff(0x1000, vec![0xca, 0x0a]);
        registers.register_x = 0x10;
        assert_eq!("Pika".to_owned(), cpu_instruction.execute(&mut memory, &mut registers));
        assert_eq!(0x0f, registers.register_x);
        assert_eq!(0b00000000, registers.status_register & 0b10000010);
    }
}
