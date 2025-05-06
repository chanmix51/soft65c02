use std::collections::{HashMap, HashSet};
use soft65c02_lib::{Memory, CPUInstruction, AddressingMode, resolve_relative};
use crate::{AppResult, SymbolTable};
use soft65c02_lib::memory::little_endian;

struct FormattedInstruction<'a> {
    instruction: &'a CPUInstruction,
    symbols: Option<&'a SymbolTable>,
    branch_labels: Option<&'a HashMap<usize, String>>,
}

impl<'a> FormattedInstruction<'a> {
    fn new(instruction: &'a CPUInstruction, symbols: Option<&'a SymbolTable>, branch_labels: Option<&'a HashMap<usize, String>>) -> Self {
        Self { instruction, symbols, branch_labels }
    }

    fn get_symbol_for_address(&self, addr: usize) -> Option<String> {
        // First check regular symbols
        if let Some(symbol) = self.symbols.and_then(|symbols| {
            symbols.get_symbols_for_address(addr as u16).first().cloned()
        }) {
            return Some(symbol);
        }
        
        // Then check branch labels
        self.branch_labels.and_then(|labels| labels.get(&addr).cloned())
    }

    fn format(&self) -> String {
        let bytes = {
            let mut bytes = vec![self.instruction.opcode];
            bytes.extend(&self.instruction.addressing_mode.get_operands());
            format!("({})", bytes.iter()
                .fold(String::new(), |acc, s| format!("{} {:02x}", acc, s))
                .trim())
        };

        let formatted = match self.instruction.addressing_mode {
            AddressingMode::Implied => {
                format!("{: <12}{: <4}", bytes, self.instruction.mnemonic)
            },
            AddressingMode::Accumulator => {
                format!("{: <12}{: <4} A", bytes, self.instruction.mnemonic)
            },
            AddressingMode::Immediate(v) => {
                format!("{: <12}{: <4} #${:02x}", bytes, self.instruction.mnemonic, v[0])
            },
            AddressingMode::ZeroPage(v) => {
                let addr = v[0] as usize;
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} {}", bytes, self.instruction.mnemonic, symbol)
                } else {
                    format!("{: <12}{: <4} ${:02x}", bytes, self.instruction.mnemonic, v[0])
                }
            },
            AddressingMode::Absolute(v) => {
                let addr = little_endian(vec![v[0], v[1]]);
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} {}", bytes, self.instruction.mnemonic, symbol)
                } else {
                    format!("{: <12}{: <4} ${:02X}{:02X}", bytes, self.instruction.mnemonic, v[1], v[0])
                }
            },
            AddressingMode::ZeroPageXIndexed(v) => {
                let addr = v[0] as usize;
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} {symbol},X", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} ${:02x},X", bytes, self.instruction.mnemonic, v[0])
                }
            },
            AddressingMode::ZeroPageYIndexed(v) => {
                let addr = v[0] as usize;
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} {symbol},Y", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} ${:02x},Y", bytes, self.instruction.mnemonic, v[0])
                }
            },
            AddressingMode::ZeroPageXIndexedIndirect(v) => {
                let addr = v[0] as usize;
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} ({symbol},X)", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} (${:02x},X)", bytes, self.instruction.mnemonic, v[0])
                }
            },
            AddressingMode::ZeroPageIndirectYIndexed(v) => {
                let addr = v[0] as usize;
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} ({symbol}),Y", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} (${:02x}),Y", bytes, self.instruction.mnemonic, v[0])
                }
            },
            AddressingMode::ZeroPageIndirect(v) => {
                let addr = v[0] as usize;
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} ({symbol})", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} (${:02x})", bytes, self.instruction.mnemonic, v[0])
                }
            },
            AddressingMode::AbsoluteXIndexed(v) => {
                let addr = little_endian(vec![v[0], v[1]]);
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} {symbol},X", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} ${:02X}{:02X},X", bytes, self.instruction.mnemonic, v[1], v[0])
                }
            },
            AddressingMode::AbsoluteYIndexed(v) => {
                let addr = little_endian(vec![v[0], v[1]]);
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} {symbol},Y", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} ${:02X}{:02X},Y", bytes, self.instruction.mnemonic, v[1], v[0])
                }
            },
            AddressingMode::Indirect(v) => {
                let addr = little_endian(vec![v[0], v[1]]);
                if let Some(symbol) = self.get_symbol_for_address(addr) {
                    format!("{: <12}{: <4} ({symbol})", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} (${:02X}{:02X})", bytes, self.instruction.mnemonic, v[1], v[0])
                }
            },
            AddressingMode::Relative(addr, v) => {
                let target_addr = resolve_relative(addr, v[0]).unwrap();
                if let Some(symbol) = self.get_symbol_for_address(target_addr) {
                    format!("{: <12}{: <4} {symbol}", bytes, self.instruction.mnemonic)
                } else {
                    format!("{: <12}{: <4} ${:04X}", bytes, self.instruction.mnemonic, target_addr)
                }
            },
            AddressingMode::ZeroPageRelative(addr, v) => {
                let zp_addr = v[0] as usize;
                let target_addr = resolve_relative(addr, v[1]).unwrap();
                let zp_symbol = self.get_symbol_for_address(zp_addr);
                let target_symbol = self.get_symbol_for_address(target_addr);
                match (zp_symbol, target_symbol) {
                    (Some(zp), Some(tgt)) => format!("{: <12}{: <4} {zp},{tgt}", bytes, self.instruction.mnemonic),
                    (Some(zp), None) => format!("{: <12}{: <4} {zp},${:04X}", bytes, self.instruction.mnemonic, target_addr),
                    (None, Some(tgt)) => format!("{: <12}{: <4} ${:02x},{tgt}", bytes, self.instruction.mnemonic, v[0]),
                    (None, None) => format!("{: <12}{: <4} ${:02x},${:04X}", bytes, self.instruction.mnemonic, v[0], target_addr),
                }
            },
            _ => {
                let this = &self;
                format!("{}", this.instruction)
            },
        };

        format!("#0x{:04X}: {}", self.instruction.address, formatted)
    }
}

pub struct Disassembler<'a> {
    memory: &'a Memory,
    symbols: &'a mut Option<SymbolTable>,
}

impl<'a> Disassembler<'a> {
    pub fn new(memory: &'a Memory, symbols: &'a mut Option<SymbolTable>) -> Self {
        Self { memory, symbols }
    }

    /// Collect branch targets and addresses with symbols from instructions
    fn collect_targets_and_symbols(
        &self,
        instructions: &[CPUInstruction],
    ) -> (HashSet<usize>, HashMap<usize, String>, HashSet<usize>) {
        let mut branch_targets = HashSet::new();
        let mut branch_labels = HashMap::new();
        let mut addresses_with_symbols = HashSet::new();
        let mut next_branch_id = 1;

        // First collect all addresses that have symbols
        if let Some(symbols) = &self.symbols {
            for instr in instructions.iter() {
                if !symbols.get_symbols_for_address(instr.address as u16).is_empty() {
                    addresses_with_symbols.insert(instr.address);
                }
            }
        }

        // Then handle branch targets that don't have symbols
        for instr in instructions.iter() {
            let instr_str = format!("{}", instr);
            if is_branch_instruction(&instr_str) {
                if let Some(target_addr) = extract_branch_target(&instr_str) {
                    branch_targets.insert(target_addr);
                    // Only create a branch label if the target doesn't have a symbol
                    if !addresses_with_symbols.contains(&target_addr) && !branch_labels.contains_key(&target_addr) {
                        branch_labels.insert(target_addr, format!("branch_{}", next_branch_id));
                        next_branch_id += 1;
                    }
                }
            }
        }

        (branch_targets, branch_labels, addresses_with_symbols)
    }

    pub fn disassemble_range(&self, start: usize, end: usize) -> AppResult<Vec<String>> {
        use soft65c02_lib::disassemble;
        let instructions = disassemble(start, end + 1, self.memory)?;
        let mut output = vec!["---- Start of disassembly ----".to_string()];
        
        // First pass: collect branch targets and symbols
        let (branch_targets, branch_labels, addresses_with_symbols) = 
            self.collect_targets_and_symbols(&instructions);

        // Second pass: Generate output with labels
        let mut last_labeled_addr = None;
        for instr in instructions {
            // Check if this instruction's address has a symbol
            if let Some(symbols) = &self.symbols {
                if last_labeled_addr != Some(instr.address) {  // Avoid duplicate labels
                    let addr_symbols = symbols.get_symbols_for_address(instr.address as u16);
                    if !addr_symbols.is_empty() {
                        output.push(format!("{}:", addr_symbols.join(", ")));
                    }
                }
            }

            // Check if this is a branch target and doesn't have a symbol
            if branch_targets.contains(&instr.address) && 
               !addresses_with_symbols.contains(&instr.address) && 
               last_labeled_addr != Some(instr.address) {
                if let Some(label) = branch_labels.get(&instr.address) {
                    output.push(format!("{}:", label));
                }
            }

            let formatted = FormattedInstruction::new(&instr, self.symbols.as_ref(), Some(&branch_labels)).format();
            output.push(formatted);
            
            last_labeled_addr = Some(instr.address);
        }
        
        output.push("----- End of disassembly -----".to_string());
        Ok(output)
    }
}

/// Check if an instruction string represents a branch instruction
fn is_branch_instruction(instruction: &str) -> bool {
    static BRANCH_OPCODES: [&str; 8] = ["BCC", "BCS", "BEQ", "BMI", "BNE", "BPL", "BVC", "BVS"];
    BRANCH_OPCODES.iter().any(|&opcode| instruction.contains(opcode))
}

/// Extract the target address from a branch instruction
fn extract_branch_target(instruction: &str) -> Option<usize> {
    // Branch instructions will have a target address in the form "$XXXX"
    if is_branch_instruction(instruction) {
        let dollar_pos = instruction.rfind('$')?;
        let addr_str = &instruction[dollar_pos + 1..];
        let addr_end = addr_str.find(|c: char| !c.is_ascii_hexdigit())
            .unwrap_or(addr_str.len());
        let addr_str = &addr_str[..addr_end];
        
        if let Ok(addr) = usize::from_str_radix(addr_str, 16) {
            return Some(addr);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use soft65c02_lib::AddressableIO;

    #[test]
    fn test_disassemble_simple_program() {
        let mut memory = Memory::new_with_ram();
        let mut symbols = None;
        
        // Write a simple program to memory
        memory.write(0x1000, &[0xa9, 0x42, 0x8d, 0x34, 0x12, 0x60]).unwrap();

        let disassembler = Disassembler::new(&memory, &mut symbols);
        let output = disassembler.disassemble_range(0x1000, 0x1005).unwrap();

        let expected_output = "\
---- Start of disassembly ----
#0x1000: (a9 42)     LDA  #$42
#0x1002: (8d 34 12)  STA  $1234
#0x1005: (60)        RTS 
----- End of disassembly -----";

        let actual_output = output.join("\n");
        assert_eq!(actual_output, expected_output, "\nExpected:\n{}\n\nActual:\n{}\n", expected_output, actual_output);
    }

    #[test]
    fn test_disassemble_with_symbols() {
        let mut memory = Memory::new_with_ram();
        let mut symbols = Some(SymbolTable::new());
        
        // Write a simple program to memory
        memory.write(0x1000, &[
            0xa9, 0x42,             // LDA #$42
            0x8d, 0x34, 0x12,       // STA $1234
            0xb1, 0x88,             // LDA ($88),Y
            0x20, 0x00, 0x20,       // JSR $2000
            0x60                    // RTS
        ]).unwrap();
        
        // Add some symbols
        if let Some(symbols) = &mut symbols {
            symbols.add_symbol(0x1000, "start".to_string());
            symbols.add_symbol(0x1000, "main".to_string());  // Multiple symbols for same address
            symbols.add_symbol(0x1234, "COUNTER".to_string());
            symbols.add_symbol(0x0088, "TMP1".to_string());
            symbols.add_symbol(0x2000, "other_func".to_string());
            // this should not match the literal value as we explicitly don't want to mix values with symbols
            symbols.add_symbol(0x42, "VAL1".to_string());
        }

        let disassembler = Disassembler::new(&memory, &mut symbols);
        let output = disassembler.disassemble_range(0x1000, 0x100a).unwrap();

        let expected_output = "\
---- Start of disassembly ----
main, start:
#0x1000: (a9 42)     LDA  #$42
#0x1002: (8d 34 12)  STA  COUNTER
#0x1005: (b1 88)     LDA  (TMP1),Y
#0x1007: (20 00 20)  JSR  other_func
#0x100A: (60)        RTS 
----- End of disassembly -----";

        let actual_output = output.join("\n");
        assert_eq!(actual_output, expected_output, "\nExpected:\n{}\n\nActual:\n{}\n", expected_output, actual_output);
    }

    #[test]
    fn test_disassemble_with_branch() {
        let mut memory = Memory::new_with_ram();
        let mut symbols = Some(SymbolTable::new());
        
        // Write a program with multiple branches
        memory.write(0x1000, &[
            0x18,             // CLC
            0xa9, 0x10,       // LDA #$10
            0x6d, 0x00, 0x20, // ADC $2000
            0x8d, 0x00, 0x20, // STA $2000
            0x90, 0x0D,       // BCC +0D (to branch_1)
            0xee, 0x01, 0x20, // INC $2001
            0xd0, 0x05,       // BNE +5 (to branch_2)
            0xf0, 0xee,       // BEQ -18 (back to start/main)
            0x4c, 0x1b, 0x10, // JMP end
            0xa9, 0x00,       // LDA #$00
            0x60,            // RTS
            0xa9, 0xff,       // LDA #$ff
            0x60,            // RTS
            0xa9, 0x42,       // LDA #$42
            0x60             // RTS (end)
        ]).unwrap();
        
        // Add symbols for memory locations, including multiple symbols for same address
        if let Some(symbols) = &mut symbols {
            symbols.add_symbol(0x1000, "start".to_string());
            symbols.add_symbol(0x1000, "main".to_string());
            symbols.add_symbol(0x2000, "mem_lo".to_string());
            symbols.add_symbol(0x2001, "mem_hi".to_string());
            symbols.add_symbol(0x101b, "end".to_string());
        }

        let disassembler = Disassembler::new(&memory, &mut symbols);
        let output = disassembler.disassemble_range(0x1000, 0x101d).unwrap();

        let expected_output = "\
---- Start of disassembly ----
main, start:
#0x1000: (18)        CLC 
#0x1001: (a9 10)     LDA  #$10
#0x1003: (6d 00 20)  ADC  mem_lo
#0x1006: (8d 00 20)  STA  mem_lo
#0x1009: (90 0d)     BCC  branch_1
#0x100B: (ee 01 20)  INC  mem_hi
#0x100E: (d0 05)     BNE  branch_2
#0x1010: (f0 ee)     BEQ  main
#0x1012: (4c 1b 10)  JMP  end
branch_2:
#0x1015: (a9 00)     LDA  #$00
#0x1017: (60)        RTS 
branch_1:
#0x1018: (a9 ff)     LDA  #$ff
#0x101A: (60)        RTS 
end:
#0x101B: (a9 42)     LDA  #$42
#0x101D: (60)        RTS 
----- End of disassembly -----";

        let actual_output = output.join("\n");
        assert_eq!(actual_output, expected_output, "\nExpected:\n{}\n\nActual:\n{}\n", expected_output, actual_output);
    }

    #[test]
    fn test_disassemble_with_indirect_addressing() {
        let mut memory = Memory::new_with_ram();
        let mut symbols = Some(SymbolTable::new());
        
        // Write a program with indirect addressing modes
        memory.write(0x1000, &[
            0xb1, 0x88,             // LDA ($88),Y
            0xa1, 0x90,             // LDA ($90,X)
            0x81, 0x92,             // STA ($92,X)
            0x91, 0x94,             // STA ($94),Y
            0x60                    // RTS
        ]).unwrap();
        
        // Add symbols for zero page locations
        if let Some(symbols) = &mut symbols {
            symbols.add_symbol(0x0088, "PTR1".to_string());
            symbols.add_symbol(0x0090, "PTR2".to_string());
            symbols.add_symbol(0x0092, "PTR3".to_string());
            symbols.add_symbol(0x0094, "PTR4".to_string());
        }

        let disassembler = Disassembler::new(&memory, &mut symbols);
        let output = disassembler.disassemble_range(0x1000, 0x1008).unwrap();

        let expected_output = "\
---- Start of disassembly ----
#0x1000: (b1 88)     LDA  (PTR1),Y
#0x1002: (a1 90)     LDA  (PTR2,X)
#0x1004: (81 92)     STA  (PTR3,X)
#0x1006: (91 94)     STA  (PTR4),Y
#0x1008: (60)        RTS 
----- End of disassembly -----";

        let actual_output = output.join("\n");
        assert_eq!(actual_output, expected_output, "\nExpected:\n{}\n\nActual:\n{}\n", expected_output, actual_output);
    }

    #[test]
    fn test_zero_page_symbol_substitution() {
        let mut memory = Memory::new_with_ram();
        let mut symbols = Some(SymbolTable::new());
        
        // Write a program with zero page instructions
        memory.write(0x1000, &[
            0xa9, 0x8a,             // LDA #$8a - should NOT be substituted
            0x85, 0x8a,             // STA $8a
            0x86, 0x92,             // STX $92
        ]).unwrap();
        
        // Add symbols for zero page locations
        if let Some(symbols) = &mut symbols {
            symbols.add_symbol(0x008a, "ptr1".to_string());
            symbols.add_symbol(0x0092, "ptr2".to_string());
        }

        let disassembler = Disassembler::new(&memory, &mut symbols);
        let output = disassembler.disassemble_range(0x1000, 0x1005).unwrap();

        let expected_output = "\
---- Start of disassembly ----
#0x1000: (a9 8a)     LDA  #$8a
#0x1002: (85 8a)     STA  ptr1
#0x1004: (86 92)     STX  ptr2
----- End of disassembly -----";

        let actual_output = output.join("\n");
        assert_eq!(actual_output, expected_output, "\nExpected:\n{}\n\nActual:\n{}\n", expected_output, actual_output);
    }
} 