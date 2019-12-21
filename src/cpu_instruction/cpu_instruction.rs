use crate::memory::RAM as Memory;
use crate::registers::Registers;
use crate::addressing_mode::*;
use std::fmt;

pub struct CPUInstruction {
    pub address:    usize,
    pub opcode:     u8,
    pub mnemonic:   String,
    pub addressing_mode: AddressingMode,
    pub microcode:  Box<dyn Fn(&mut Memory, &mut Registers, &CPUInstruction) -> LogLine>,
}

impl CPUInstruction {
    pub fn new(address: usize, opcode: u8, mnemonic: &str, addressing_mode: AddressingMode, microcode: impl Fn(&mut Memory, &mut Registers, &CPUInstruction) -> LogLine + 'static) -> CPUInstruction {
        CPUInstruction {
            address:            address,
            opcode:             opcode,
            mnemonic:           mnemonic.to_owned(),
            addressing_mode:    addressing_mode,
            microcode:          Box::new(microcode)
        }
    }

    pub fn execute(&self, memory: &mut Memory, registers: &mut Registers) -> LogLine {
        (self.microcode)(memory, registers, &self)
    }

    pub fn simulate(&self, memory: &Memory, registers: &Registers) -> LogLine {
       let resolution = self.addressing_mode.solve(self.address, memory, registers);
       LogLine {
            address:    self.address,
            opcode:     self.opcode,
            mnemonic:   self.mnemonic.clone(),
            resolution: resolution,
       }
    }
}

pub struct LogLine {
    pub address:    usize,
    pub opcode:     u8,
    pub mnemonic:   String,
    pub resolution: AddressingModeResolution,
}

impl fmt::Display for LogLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = vec![self.opcode];
        for i in self.resolution.operands.clone() { bytes.push(i); }
        let byte_sequence = format!("({})", bytes.iter().fold(String::new(), |acc, s| format!("{} {:02x}", acc, s)).trim());
        let dest_addr = match self.resolution.target_address {
            Some(addr)  => format!("(#0x{:04X})", addr),
            None        => String::new(),
        };

        write!(f, "#0x{:04X}: {: <10} {: <4} {: <10} {} ", self.address, byte_sequence, self.mnemonic, self.resolution, dest_addr)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn get_stuff(addr: usize, program: Vec<u8>) -> (Memory, Registers) {
        let mut memory = Memory::new();
        memory.write(addr, program);
        let mut registers = Registers::new(addr);

        (memory, registers)
    }
}

