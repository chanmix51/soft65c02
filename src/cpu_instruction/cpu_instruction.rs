use crate::memory::MemoryStack as Memory;
use crate::registers::Registers;
use crate::addressing_mode::*;
use crate::cpu_instruction::microcode::Result as MicrocodeResult;
use std::fmt;

pub struct CPUInstruction {
    pub address:    usize,
    pub opcode:     u8,
    pub mnemonic:   String,
    pub addressing_mode: AddressingMode,
    pub microcode:  Box<dyn Fn(&mut Memory, &mut Registers, &CPUInstruction) -> MicrocodeResult<LogLine>>,
}

impl CPUInstruction {
    pub fn new(
    address: usize,
    opcode: u8,
    mnemonic: &str,
    addressing_mode: AddressingMode,
    microcode: impl Fn(&mut Memory, &mut Registers, &CPUInstruction) -> MicrocodeResult<LogLine> + 'static
    ) -> CPUInstruction {
        CPUInstruction {
            address:            address,
            opcode:             opcode,
            mnemonic:           mnemonic.to_owned(),
            addressing_mode:    addressing_mode,
            microcode:          Box::new(microcode)
        }
    }

    pub fn execute(&self, memory: &mut Memory, registers: &mut Registers) -> MicrocodeResult<LogLine> {
        (self.microcode)(memory, registers, &self)
    }
}

impl fmt::Display for CPUInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = vec![self.opcode];

        for i in self.addressing_mode.get_operands() { bytes.push(i); }
        let byte_sequence = format!("({})", bytes.iter().fold(String::new(), |acc, s| format!("{} {:02x}", acc, s)).trim());

        write!(f, "#0x{:04X}: {: <14}{: <4} {: <15}", self.address, byte_sequence, self.mnemonic, self.addressing_mode)
    }
}

#[derive(Debug)]
pub struct LogLine {
    pub address:    usize,
    pub opcode:     u8,
    pub mnemonic:   String,
    pub resolution: AddressingModeResolution,
    pub is_simulated: bool,
}

impl fmt::Display for LogLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = vec![self.opcode];
        for i in self.resolution.operands.clone() { bytes.push(i); }
        let byte_sequence = format!("({})", bytes.iter().fold(String::new(), |acc, s| format!("{} {:02x}", acc, s)).trim());

        write!(f, "#0x{:04X}: {: <14}{: <4} {: <15}", self.address, byte_sequence, self.mnemonic, self.resolution)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::memory::AddressableIO;

    pub fn get_stuff(addr: usize, program: Vec<u8>) -> (Memory, Registers) {
        let mut memory = Memory::new_with_ram();
        memory.write(addr, program).unwrap();
        let registers = Registers::new(addr);

        (memory, registers)
    }
}
