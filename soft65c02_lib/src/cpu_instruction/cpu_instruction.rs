use super::microcode::Result as MicrocodeResult;
use crate::addressing_mode::*;
use crate::memory::MemoryStack as Memory;
use crate::registers::Registers;
use std::fmt;
use std::cell::Cell;

/// Cycle timings for the 65C02 instructions
/// Values taken from Symon emulator's CMOS timing table
/// https://raw.githubusercontent.com/sethm/symon/refs/heads/master/src/main/java/com/loomcom/symon/InstructionTable.java
const INSTRUCTION_CYCLES: [u8; 256] = [
    7, 6, 2, 1, 5, 3, 5, 5, 3, 2, 2, 1, 6, 4, 6, 5, // 0x00-0x0f
    2, 5, 5, 1, 5, 4, 6, 5, 2, 4, 2, 1, 6, 4, 6, 5, // 0x10-0x1f
    6, 6, 2, 1, 3, 3, 5, 5, 4, 2, 2, 1, 4, 4, 6, 5, // 0x20-0x2f
    2, 5, 5, 1, 4, 4, 6, 5, 2, 4, 2, 1, 4, 4, 6, 5, // 0x30-0x3f
    6, 6, 2, 1, 2, 3, 5, 3, 3, 2, 2, 1, 3, 4, 6, 5, // 0x40-0x4f
    2, 5, 5, 1, 4, 4, 6, 5, 2, 4, 3, 1, 8, 4, 6, 5, // 0x50-0x5f
    6, 6, 2, 1, 3, 3, 5, 5, 4, 2, 2, 1, 6, 4, 6, 5, // 0x60-0x6f
    2, 5, 5, 1, 4, 4, 6, 5, 2, 4, 4, 3, 6, 4, 6, 5, // 0x70-0x7f
    3, 6, 2, 1, 3, 3, 3, 5, 2, 2, 2, 1, 4, 4, 4, 5, // 0x80-0x8f
    2, 6, 5, 1, 4, 4, 4, 5, 2, 5, 2, 1, 4, 5, 5, 5, // 0x90-0x9f
    2, 6, 2, 1, 3, 3, 3, 5, 2, 2, 2, 1, 4, 4, 4, 5, // 0xa0-0xaf
    2, 5, 5, 1, 4, 4, 4, 5, 2, 4, 2, 1, 4, 4, 4, 5, // 0xb0-0xbf
    2, 6, 2, 1, 3, 3, 5, 5, 2, 2, 2, 3, 4, 4, 6, 5, // 0xc0-0xcf
    2, 5, 5, 1, 4, 4, 6, 5, 2, 4, 3, 3, 4, 4, 7, 5, // 0xd0-0xdf
    2, 6, 2, 1, 3, 3, 5, 5, 2, 2, 2, 1, 4, 4, 6, 5, // 0xe0-0xef
    2, 5, 5, 1, 4, 4, 6, 5, 2, 4, 4, 1, 4, 4, 7, 5  // 0xf0-0xff
];

pub type BoxedMicrocode =
    Box<dyn Fn(&mut Memory, &mut Registers, &CPUInstruction) -> MicrocodeResult<LogLine>>;
pub struct CPUInstruction {
    pub address: usize,
    pub opcode: u8,
    pub mnemonic: String,
    pub addressing_mode: AddressingMode,
    pub microcode: BoxedMicrocode,
    pub cycles: Cell<u8>,
}

impl CPUInstruction {
    pub fn new(
        address: usize,
        opcode: u8,
        mnemonic: &str,
        addressing_mode: AddressingMode,
        microcode: impl Fn(&mut Memory, &mut Registers, &CPUInstruction) -> MicrocodeResult<LogLine>
            + 'static,
    ) -> CPUInstruction {
        CPUInstruction {
            address,
            opcode,
            mnemonic: mnemonic.to_owned(),
            addressing_mode,
            microcode: Box::new(microcode),
            cycles: Cell::new(INSTRUCTION_CYCLES[opcode as usize]),
        }
    }

    pub fn execute(
        &self,
        memory: &mut Memory,
        registers: &mut Registers,
    ) -> MicrocodeResult<LogLine> {
        (self.microcode)(memory, registers, self)
    }

    // Add branch cycles based on whether branch was taken and page boundary crossed
    pub fn add_branch_cycles(&self, registers: &Registers, original_cp: usize) {
        if let AddressingMode::Relative(_, _) = self.addressing_mode {
            let next_instruction = original_cp + 2;
            
            // If branch was taken (command pointer changed from next instruction)
            if registers.command_pointer != next_instruction {
                // Add one cycle for taken branch
                self.cycles.set(self.cycles.get() + 1);
                
                // Add another cycle if page boundary crossed
                if next_instruction & 0xFF00 != registers.command_pointer & 0xFF00 {
                    self.cycles.set(self.cycles.get() + 1);
                }
            }
        }
    }

    pub fn adjust_base_cycles(&self, registers: &Registers, memory: &Memory) {
        if self.addressing_mode.needs_page_crossing_cycle(registers, memory) {
            self.cycles.set(self.cycles.get() + 1);
        }
    }
}

impl fmt::Display for CPUInstruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = vec![self.opcode];

        for i in self.addressing_mode.get_operands() {
            bytes.push(i);
        }
        let byte_sequence = format!(
            "({})",
            bytes
                .iter()
                .fold(String::new(), |acc, s| format!("{} {:02x}", acc, s))
                .trim()
        );

        write!(
            f,
            "#0x{:04X}: {: <14}{: <4} {: <15}",
            self.address, byte_sequence, self.mnemonic, self.addressing_mode
        )
    }
}

#[derive(Debug)]
pub struct LogLine {
    pub address: usize,
    pub opcode: u8,
    pub mnemonic: String,
    pub resolution: AddressingModeResolution,
    pub outcome: String,
    pub cycles: u8,
}

impl LogLine {
    pub fn new(
        cpu_instruction: &CPUInstruction,
        resolution: AddressingModeResolution,
        outcome: String,
    ) -> LogLine {
        LogLine {
            address: cpu_instruction.address,
            opcode: cpu_instruction.opcode,
            mnemonic: cpu_instruction.mnemonic.clone(),
            resolution,
            outcome,
            cycles: cpu_instruction.cycles.get(),
        }
    }
}

impl fmt::Display for LogLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut bytes = vec![self.opcode];
        for i in self.resolution.operands.clone() {
            bytes.push(i);
        }
        let byte_sequence = format!(
            "({})",
            bytes
                .iter()
                .fold(String::new(), |acc, s| format!("{} {:02x}", acc, s))
                .trim()
        );

        write!(
            f,
            "#0x{:04X}: {: <14}{: <4} {: <15}  {}[{}]",
            self.address, byte_sequence, self.mnemonic, self.resolution, self.outcome, self.cycles
        )
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::memory::AddressableIO;
    use crate::processing_unit::resolve_opcode;

    pub fn get_stuff(addr: usize, program: Vec<u8>) -> (Memory, Registers) {
        let mut memory = Memory::new_with_ram();
        memory.write(addr, &program).unwrap();
        let registers = Registers::new_initialized(addr);

        (memory, registers)
    }

    #[test]
    fn test_instruction_cycles() {
        let mut memory = Memory::new_with_ram();
        
        // Write test instructions to memory
        memory.write(0x1000, &[0xa9, 0x00]).unwrap(); // LDA #$nn
        let lda_imm = resolve_opcode(0x1000, 0xa9, &memory).unwrap();
        assert_eq!(lda_imm.cycles.get(), 2, "LDA immediate should take 2 cycles");

        memory.write(0x1000, &[0x8d, 0x00, 0x20]).unwrap(); // STA $nnnn
        let sta_abs = resolve_opcode(0x1000, 0x8d, &memory).unwrap();
        assert_eq!(sta_abs.cycles.get(), 4, "STA absolute should take 4 cycles");

        memory.write(0x1000, &[0x20, 0x00, 0x20]).unwrap(); // JSR $nnnn
        let jsr_abs = resolve_opcode(0x1000, 0x20, &memory).unwrap();
        assert_eq!(jsr_abs.cycles.get(), 6, "JSR absolute should take 6 cycles");

        memory.write(0x1000, &[0x60]).unwrap(); // RTS implied
        let rts = resolve_opcode(0x1000, 0x60, &memory).unwrap();
        assert_eq!(rts.cycles.get(), 6, "RTS should take 6 cycles");

        memory.write(0x1000, &[0x00]).unwrap(); // BRK implied
        let brk = resolve_opcode(0x1000, 0x00, &memory).unwrap();
        assert_eq!(brk.cycles.get(), 7, "BRK should take 7 cycles");

        memory.write(0x1000, &[0xdb]).unwrap(); // STP implied
        let stp = resolve_opcode(0x1000, 0xdb, &memory).unwrap();
        assert_eq!(stp.cycles.get(), 3, "STP should take 3 cycles");
    }
}
