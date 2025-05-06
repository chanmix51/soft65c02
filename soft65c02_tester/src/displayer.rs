use std::{io::Write, sync::mpsc::Receiver};
use soft65c02_lib::{LogLine, AddressingMode};
use crate::{AppResult, OutputToken, SymbolTable, commands::ControllableFunction};

struct LogLineFormatter<'a> {
    log_line: &'a LogLine,
    symbols: Option<&'a SymbolTable>,
}

impl<'a> LogLineFormatter<'a> {
    fn new(log_line: &'a LogLine, symbols: Option<&'a SymbolTable>) -> Self {
        Self { log_line, symbols }
    }

    fn get_adjacent_symbol(&self, addr: u16) -> Option<String> {
        const MAX_STRUCT_OFFSET: u16 = 0x1F;  // Maximum reasonable struct field offset

        if let Some(symbols) = self.symbols {
            // First check if we have an exact match
            if !symbols.get_symbols_for_address(addr).is_empty() {
                return None;  // Let the exact match be handled by the caller
            }

            // Look for the closest symbol before this address within our max offset
            let mut closest_addr = None;
            let mut closest_distance = MAX_STRUCT_OFFSET + 1;  // Initialize to just over our max

            // Check all symbols that could be within range
            for base_addr in addr.saturating_sub(MAX_STRUCT_OFFSET)..=addr {
                if let Some(sym) = symbols.get_symbols_for_address(base_addr).first() {
                    let distance = addr - base_addr;
                    if distance > 0 && distance <= MAX_STRUCT_OFFSET && distance < closest_distance {
                        closest_addr = Some((base_addr, sym.to_string()));
                        closest_distance = distance;
                    }
                }
            }

            // If we found a close symbol, format it with the offset
            if let Some((base_addr, sym)) = closest_addr {
                let offset = addr - base_addr;
                return Some(format!("{}+{}", sym, offset));
            }
        }
        None
    }

    fn format_addressing_mode(&self, total_width: usize) -> String {
        const ADDR_WIDTH: usize = 9; // (#0xXXXX) is always 9 chars
        let content_width = total_width.saturating_sub(ADDR_WIDTH + 1); // +1 for space before address
        
        let mode_str = format!("{}", self.log_line.resolution.addressing_mode);
        
        // Get symbol for the final target address, or base address for indexed modes
        let target_symbol = match self.log_line.resolution.addressing_mode {
            AddressingMode::AbsoluteXIndexed([lo, hi]) | AddressingMode::AbsoluteYIndexed([lo, hi]) => {
                let base_addr = ((hi as u16) << 8) | (lo as u16);
                self.symbols.and_then(|symbols| {
                    symbols.get_symbols_for_address(base_addr).first().map(|s| s.to_string())
                })
            },
            _ => self.log_line.resolution.target_address.and_then(|addr| {
                // First try direct symbol lookup
                if let Some(sym) = self.symbols.and_then(|symbols| {
                    symbols.get_symbols_for_address(addr as u16).first().map(|s| s.to_string())
                }) {
                    Some(sym)
                } else {
                    // If no direct symbol, try adjacent symbol for zero page
                    self.get_adjacent_symbol(addr as u16)
                }
            })
        };

        // Format the address part
        let addr_str = self.log_line.resolution.target_address
            .map(|addr| format!("(#0x{:04X})", addr));

        // Special handling for indirect addressing modes
        match self.log_line.resolution.addressing_mode {
            AddressingMode::ZeroPageIndirectYIndexed(v) | 
            AddressingMode::ZeroPageXIndexedIndirect(v) |
            AddressingMode::ZeroPageIndirect(v) => {
                // Try to get symbol for the base pointer address
                let base_symbol = self.symbols.and_then(|symbols| {
                    symbols.get_symbols_for_address(v[0] as u16).first().map(|s| s.to_string())
                });

                if let Some(sym) = base_symbol {
                    // Format with the symbol replacing the hex address
                    let mode_str = match self.log_line.resolution.addressing_mode {
                        AddressingMode::ZeroPageIndirectYIndexed(_) => format!("({sym}),Y"),
                        AddressingMode::ZeroPageXIndexedIndirect(_) => format!("({sym},X)"),
                        AddressingMode::ZeroPageIndirect(_) => format!("({sym})"),
                        _ => unreachable!()
                    };
                    format!("{:<width$} {}", 
                        mode_str,
                        addr_str.unwrap_or_default(),
                        width = content_width
                    )
                } else {
                    // No symbol for base pointer, use original format
                    format!("{:<width$} {}", 
                        mode_str,
                        addr_str.unwrap_or_default(),
                        width = content_width
                    )
                }
            },
            _ => {
                match (target_symbol, addr_str) {
                    (Some(sym), Some(addr)) => {
                        // If we have both symbol and address
                        match self.log_line.resolution.addressing_mode {
                            AddressingMode::Immediate(_) => {
                                // For immediate mode, keep the #$ prefix
                                format!("{} {:<width$} {}", 
                                    mode_str, 
                                    "", 
                                    addr,
                                    width = content_width.saturating_sub(mode_str.len() + 1)
                                )
                            },
                            AddressingMode::AbsoluteXIndexed(_) => {
                                format!("{:<width$} {}", 
                                    format!("{},X", sym),
                                    addr,
                                    width = content_width
                                )
                            },
                            AddressingMode::AbsoluteYIndexed(_) => {
                                format!("{:<width$} {}", 
                                    format!("{},Y", sym),
                                    addr,
                                    width = content_width
                                )
                            },
                            _ => {
                                // For other modes, just show the symbol
                                format!("{:<width$} {}", 
                                    sym, 
                                    addr,
                                    width = content_width
                                )
                            }
                        }
                    },
                    (None, Some(addr)) => {
                        // If we only have an address
                        if mode_str.is_empty() {
                            format!("{:>width$} {}", 
                                "", 
                                addr,
                                width = content_width
                            )
                        } else {
                            format!("{:<width$} {}", 
                                mode_str, 
                                addr,
                                width = content_width
                            )
                        }
                    },
                    (Some(sym), None) => {
                        // If we only have a symbol (unusual case)
                        format!("{:<width$}", 
                            sym,
                            width = total_width
                        )
                    },
                    (None, None) => {
                        // If we have neither (e.g., implied addressing)
                        format!("{:<width$}", 
                            mode_str,
                            width = total_width
                        )
                    }
                }
            }
        }
    }

    fn format(&self) -> String {
        // Format the byte sequence
        let mut bytes = vec![self.log_line.opcode];
        bytes.extend(&self.log_line.resolution.operands);
        let byte_sequence = format!(
            "({})",
            bytes
                .iter()
                .fold(String::new(), |acc, s| format!("{} {:02x}", acc, s))
                .trim()
        );

        // Format register state
        let register_state = format!(
            "{:02X}|{:02X}|{:02X}|{:02X}|{}",
            self.log_line.registers.accumulator,
            self.log_line.registers.register_x,
            self.log_line.registers.register_y,
            self.log_line.registers.stack_pointer,
            self.log_line.registers.format_status()
        );

        // Format the final output with debug markers
        format!(
            "#0x{:04X}: {: <12}{: <4} {: <25} {}[{}]",
            self.log_line.address,
            byte_sequence,
            self.log_line.mnemonic,
            self.format_addressing_mode(25),
            register_state,
            self.log_line.cycles
        )
    }
}

pub trait Displayer {
    fn display(&mut self, receiver: Receiver<OutputToken>) -> AppResult<()>;
}

#[derive(Debug)]
pub struct CliDisplayer<T>
where
    T: Write,
{
    output: T,
    verbose: bool,
    trace_logging_enabled: bool,
}

impl<T> Default for CliDisplayer<T>
where
    T: Write + Default,
{
    fn default() -> Self {
        Self {
            output: T::default(),
            verbose: false,
            trace_logging_enabled: true,
        }
    }
}

impl<T> CliDisplayer<T>
where
    T: Write,
{
    pub fn new(output: T, verbose: bool) -> Self {
        Self { 
            output, 
            verbose, 
            trace_logging_enabled: true  // Default to enabled
        }
    }

    /// Check if trace logging should be shown (both verbose and trace logging must be enabled)
    fn should_show_trace(&self) -> bool {
        self.verbose && self.trace_logging_enabled
    }
}

impl<T> Displayer for CliDisplayer<T>
where
    T: Write + Sync + Send,
{
    fn display(&mut self, receiver: Receiver<OutputToken>) -> AppResult<()> {
        let mut i: u32 = 0;

        while let Ok(token) = receiver.recv() {
            match &token {
                OutputToken::Assertion {
                    failure,
                    description,
                } => {
                    i += 1;
                    self.output.write_all(
                        format!(
                            "⚡ {i:02} → {description} {}\n",
                            match failure {
                                None => "✅".to_string(),
                                Some(msg) => format!("❌ ({msg})"),
                            }
                        )
                        .as_bytes(),
                    )?;
                }
                OutputToken::Marker { description } => {
                    self.output
                        .write_all(format!("📄 {description}\n").as_bytes())?;
                }
                OutputToken::Run { loglines, symbols } | OutputToken::TerminatedRun { loglines, symbols, .. } if self.should_show_trace() => {
                    let mut content = String::new();
                    let show_total = loglines.len() > 1;
                    let total_cycles: u32 = if show_total {
                        loglines.iter().map(|l| l.cycles as u32).sum()
                    } else {
                        0
                    };

                    for line in loglines {
                        let formatted = LogLineFormatter::new(&line, symbols.as_ref()).format();
                        content.push_str(&format!("🚀 {}\n", formatted));
                    }

                    if show_total {
                        content.push_str(&format!("🕒 Total cycles: {}\n", total_cycles));
                    }

                    if let OutputToken::TerminatedRun { reason, .. } = &token {
                        content.push_str(&format!("⛔ Run terminated: {}\n", reason));
                    }
                    
                    self.output.write_all(content.as_bytes())?;
                }
                OutputToken::Setup(lines) if self.verbose => {
                    self.output
                        .write_all(format!("🔧 {}\n", lines.join("\n")).as_bytes())?;
                }
                OutputToken::ControlAction { function, enabled } => {
                    // Handle control actions with type safety
                    match function {
                        ControllableFunction::TraceLogging => {
                            self.trace_logging_enabled = *enabled;
                        }
                    }
                    
                    // Show control action messages if verbose is enabled
                    if self.verbose {
                        let action = if *enabled { "enabled" } else { "disabled" };
                        self.output
                            .write_all(format!("🔧 {} {}\n", function, action).as_bytes())?;
                    }
                }
                OutputToken::View(lines) if self.verbose => {
                    for line in lines {
                        self.output.write_all(format!("🔍 {}\n", line).as_bytes())?;
                    }
                }
                _ => (),
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soft65c02_lib::{LogLine, AddressingMode, AddressingModeResolution, RegisterState};
    use std::sync::mpsc::channel;

    #[test]
    fn test_symbol_substitution() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x02C6, "COLOR2".to_string());
        symbols.add_symbol(0x02C8, "COLOR4".to_string());

        // Create a LogLine that references one of these addresses
        let log_line = LogLine {
            address: 0x2002,
            opcode: 0x8d,
            mnemonic: "STA".to_string(),
            resolution: AddressingModeResolution {
                target_address: Some(0x02C6),
                operands: vec![0xC6, 0x02],
                addressing_mode: AddressingMode::Absolute([0xC6, 0x02]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 4,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2002,
            },
        };

        let formatted = LogLineFormatter::new(&log_line, Some(&symbols)).format();
        assert!(formatted.contains("COLOR2"), "Symbol substitution failed");
        assert_eq!(formatted, "#0x2002: (8d c6 02)  STA  COLOR2          (#0x02C6) 42|00|00|FF|nv-Bdizc[4]");

        // Test indexed addressing mode with symbol substitution
        let log_line_indexed = LogLine {
            address: 0x2002,
            opcode: 0x9d,
            mnemonic: "STA".to_string(),
            resolution: AddressingModeResolution {
                target_address: Some(0x02C7),  // Base + X
                operands: vec![0xC6, 0x02],
                addressing_mode: AddressingMode::AbsoluteXIndexed([0xC6, 0x02]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 5,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 1,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2002,
            },
        };

        let formatted = LogLineFormatter::new(&log_line_indexed, Some(&symbols)).format();
        assert!(formatted.contains("COLOR2,X"), "Indexed symbol substitution failed");
        assert_eq!(formatted, "#0x2002: (9d c6 02)  STA  COLOR2,X        (#0x02C7) 42|01|00|FF|nv-Bdizc[5]");
    }

    #[test]
    fn test_adjacent_symbol_substitution() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x008A, "ptr1".to_string());
        // Deliberately not adding a symbol for 0x8B

        // Create a LogLine that references the address after ptr1
        let log_line = LogLine {
            address: 0x2027,
            opcode: 0x85,
            mnemonic: "STA".to_string(),
            resolution: AddressingModeResolution {
                target_address: Some(0x008B),
                operands: vec![0x8B],
                addressing_mode: AddressingMode::ZeroPage([0x8B]),
            },
            outcome: "(0x20)".to_string(),
            cycles: 3,
            registers: RegisterState {
                accumulator: 0x20,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2027,
            },
        };

        let formatted = LogLineFormatter::new(&log_line, Some(&symbols)).format();
        assert!(formatted.contains("ptr1+1"), "Adjacent symbol substitution failed");
        assert_eq!(formatted, "#0x2027: (85 8b)     STA  ptr1+1          (#0x008B) 20|00|00|FF|nv-Bdizc[3]");

        // Test that it doesn't substitute if the address has its own symbol
        symbols.add_symbol(0x008B, "ptr2".to_string());
        let formatted = LogLineFormatter::new(&log_line, Some(&symbols)).format();
        assert!(formatted.contains("ptr2"), "Direct symbol should take precedence");
        assert!(!formatted.contains("ptr1+1"), "Adjacent symbol should not be used when direct symbol exists");
    }

    #[test]
    fn test_displayer() {
        // Create a simple buffer to capture output
        let mut buffer = Vec::new();
        let mut displayer = CliDisplayer::new(&mut buffer, true);

        // Create symbol table
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x02C6, "COLOR2".to_string());
        symbols.add_symbol(0x02C8, "COLOR4".to_string());

        // Create a channel just for passing the test token
        let (sender, receiver) = channel();

        // Send a run with a LogLine that should have symbol substitution
        let log_line = LogLine {
            address: 0x2027,
            opcode: 0x8d,
            mnemonic: "STA".to_string(),
            resolution: AddressingModeResolution {
                target_address: Some(0x02C8),
                operands: vec![0xC8, 0x02],
                addressing_mode: AddressingMode::Absolute([0xC8, 0x02]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 4,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2027,
            },
        };
        sender.send(OutputToken::Run { loglines: vec![log_line], symbols: Some(symbols) }).unwrap();
        
        // Drop the sender so the receiver knows there will be no more messages
        drop(sender);
        
        // Process the tokens
        displayer.display(receiver).unwrap();

        // Check the output
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("COLOR4"), "Symbol substitution not found in output");
        assert_eq!(output, "🚀 #0x2027: (8d c8 02)  STA  COLOR4          (#0x02C8) 42|00|00|FF|nv-Bdizc[4]\n");
    }

    #[test]
    fn test_terminated_run_single_instruction() {
        let mut buffer = Vec::new();
        let mut displayer = CliDisplayer::new(&mut buffer, true);
        let (sender, receiver) = channel();

        // Create a simple log line
        let log_line = LogLine {
            address: 0x2000,
            opcode: 0xa9,
            mnemonic: "LDA".to_string(),
            resolution: AddressingModeResolution {
                target_address: None,
                operands: vec![0x42],
                addressing_mode: AddressingMode::Immediate([0x42]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 2,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2002,
            },
        };

        // Send a TerminatedRun token
        sender.send(OutputToken::TerminatedRun { 
            loglines: vec![log_line], 
            symbols: None,
            reason: "Cycle count limit exceeded".to_string(),
        }).unwrap();
        drop(sender);
        
        displayer.display(receiver).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        let expected_instruction = "#0x2000: (a9 42)     LDA  #$42                      42|00|00|FF|nv-Bdizc[2]";
        // Should show the instruction
        assert!(output.contains(expected_instruction), 
            "Missing instruction trace.\nExpected to find '{}'\nActual output:\n{}", 
            expected_instruction, output);
        // Should show the termination reason
        assert!(output.contains("⛔ Run terminated: Cycle count limit exceeded"), "Missing termination reason");
        // Should not show total cycles for single instruction
        assert!(!output.contains("Total cycles:"), "Should not show total cycles for single instruction");
    }

    #[test]
    fn test_terminated_run_multiple_instructions() {
        let mut buffer = Vec::new();
        let mut displayer = CliDisplayer::new(&mut buffer, true);
        let (sender, receiver) = channel();

        // Create two log lines
        let log_line1 = LogLine {
            address: 0x2000,
            opcode: 0xa9,
            mnemonic: "LDA".to_string(),
            resolution: AddressingModeResolution {
                target_address: None,
                operands: vec![0x42],
                addressing_mode: AddressingMode::Immediate([0x42]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 2,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2002,
            },
        };

        let log_line2 = LogLine {
            address: 0x2002,
            opcode: 0x85,
            mnemonic: "STA".to_string(),
            resolution: AddressingModeResolution {
                target_address: Some(0x80),
                operands: vec![0x80],
                addressing_mode: AddressingMode::ZeroPage([0x80]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 3,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2004,
            },
        };

        // Send a TerminatedRun token with multiple instructions
        sender.send(OutputToken::TerminatedRun { 
            loglines: vec![log_line1, log_line2], 
            symbols: None,
            reason: "Cycle count limit exceeded".to_string(),
        }).unwrap();
        drop(sender);
        
        displayer.display(receiver).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // The actual formatting includes target addresses and proper spacing
        let expected_instruction1 = "#0x2000: (a9 42)     LDA  #$42                      42|00|00|FF|nv-Bdizc[2]";
        let expected_instruction2 = "#0x2002: (85 80)     STA  $80             (#0x0080) 42|00|00|FF|nv-Bdizc[3]";
        
        // Should show both instructions
        assert!(output.contains(expected_instruction1), 
            "Missing first instruction.\nExpected to find '{}'\nActual output:\n{}", 
            expected_instruction1, output);
        assert!(output.contains(expected_instruction2), 
            "Missing second instruction.\nExpected to find '{}'\nActual output:\n{}", 
            expected_instruction2, output);
        // Should show total cycles
        assert!(output.contains("🕒 Total cycles: 5"), "Missing or incorrect total cycles");
        // Should show termination reason
        assert!(output.contains("⛔ Run terminated: Cycle count limit exceeded"), "Missing termination reason");
    }

    #[test]
    fn test_terminated_run_with_symbols() {
        let mut buffer = Vec::new();
        let mut displayer = CliDisplayer::new(&mut buffer, true);
        let (sender, receiver) = channel();

        // Create symbol table
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x0080, "RESULT".to_string());

        // Create log lines that use the symbol
        let log_line = LogLine {
            address: 0x2000,
            opcode: 0x85,
            mnemonic: "STA".to_string(),
            resolution: AddressingModeResolution {
                target_address: Some(0x80),
                operands: vec![0x80],
                addressing_mode: AddressingMode::ZeroPage([0x80]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 3,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2002,
            },
        };

        // Send a TerminatedRun token with symbols
        sender.send(OutputToken::TerminatedRun { 
            loglines: vec![log_line], 
            symbols: Some(symbols),
            reason: "Cycle count limit exceeded".to_string(),
        }).unwrap();
        drop(sender);
        
        displayer.display(receiver).unwrap();

        let output = String::from_utf8(buffer).unwrap();

        // The actual formatting includes target address and proper spacing
        let expected_instruction = "#0x2000: (85 80)     STA  RESULT          (#0x0080) 42|00|00|FF|nv-Bdizc[3]";
        // Should show the instruction with symbol substitution
        assert!(output.contains(expected_instruction), 
            "Incorrect symbol formatting.\nExpected to find '{}'\nActual output:\n{}", 
            expected_instruction, output);
        // Should show termination reason
        assert!(output.contains("⛔ Run terminated: Cycle count limit exceeded"), "Missing termination reason");
    }

    #[test]
    fn test_trace_logging_control() {
        let mut buffer = Vec::new();
        let mut displayer = CliDisplayer::new(&mut buffer, true);
        let (sender, receiver) = channel();

        // Initially trace logging should be enabled
        assert!(displayer.should_show_trace());

        // Send a disable trace_logging command
        sender.send(OutputToken::ControlAction { 
            function: ControllableFunction::TraceLogging, 
            enabled: false 
        }).unwrap();

        // Send a run token that normally would be displayed
        let log_line = LogLine {
            address: 0x2000,
            opcode: 0xa9,
            mnemonic: "LDA".to_string(),
            resolution: AddressingModeResolution {
                target_address: None,
                operands: vec![0x42],
                addressing_mode: AddressingMode::Immediate([0x42]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 2,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x2002,
            },
        };
        sender.send(OutputToken::Run { loglines: vec![log_line.clone()], symbols: None }).unwrap();

        // Send an enable trace_logging command
        sender.send(OutputToken::ControlAction { 
            function: ControllableFunction::TraceLogging, 
            enabled: true 
        }).unwrap();

        // Send another run token that should now be displayed
        sender.send(OutputToken::Run { loglines: vec![log_line], symbols: None }).unwrap();
        
        drop(sender);
        displayer.display(receiver).unwrap();

        let output = String::from_utf8(buffer).unwrap();
        
        // Should show the control action messages (since verbose is true)
        assert!(output.contains("🔧 trace_logging disabled"));
        assert!(output.contains("🔧 trace_logging enabled"));
        
        // Should only show one run output (the second one, after re-enabling)
        let run_count = output.matches("🚀 #0x2000:").count();
        assert_eq!(run_count, 1, "Expected exactly one run output, got {}", run_count);
    }

    #[test]
    fn test_trace_logging_disabled_by_default_when_not_verbose() {
        let mut buffer = Vec::new();
        let displayer = CliDisplayer::new(&mut buffer, false); // verbose = false

        // Even though trace_logging_enabled is true, should_show_trace should be false
        assert!(!displayer.should_show_trace());
    }

    #[test]
    fn test_end_to_end_trace_control_integration() {
        use crate::pest_parser::CliCommandParser;
        use crate::commands::Command;
        use soft65c02_lib::{Memory, Registers, AddressableIO};
        use std::sync::mpsc::channel;

        // Set up initial state
        let mut buffer = Vec::new();
        let mut displayer = CliDisplayer::new(&mut buffer, true);
        let (sender, receiver) = channel();

        // Simulate the complete flow: DSL → Parser → Command → OutputToken → Displayer
        
        // 1. Set up memory and registers (simulating what would happen in normal execution)
        let mut memory = Memory::new_with_ram();
        let mut registers = Registers::new(0x1000);
        let mut symbols = None;
        
        // Write a simple LDA instruction
        memory.write(0x1000, &[0xa9, 0x42]).unwrap(); // LDA #$42
        
        // 2. Test the full pipeline: "disable trace_logging"
        let disable_cmd = CliCommandParser::from("disable trace_logging").unwrap();
        let disable_token = disable_cmd.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        sender.send(disable_token).unwrap();
        
        // 3. Create a run token that should NOT be displayed due to disabled tracing
        let log_line = LogLine {
            address: 0x1000,
            opcode: 0xa9,
            mnemonic: "LDA".to_string(),
            resolution: AddressingModeResolution {
                target_address: None,
                operands: vec![0x42],
                addressing_mode: AddressingMode::Immediate([0x42]),
            },
            outcome: "(0x42)".to_string(),
            cycles: 2,
            registers: RegisterState {
                accumulator: 0x42,
                register_x: 0,
                register_y: 0,
                status: 0,
                stack_pointer: 0xFF,
                command_pointer: 0x1002,
            },
        };
        sender.send(OutputToken::Run { loglines: vec![log_line.clone()], symbols: None }).unwrap();
        
        // 4. Test the full pipeline: "enable trace_logging"
        let enable_cmd = CliCommandParser::from("enable trace_logging").unwrap();
        let enable_token = enable_cmd.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        sender.send(enable_token).unwrap();
        
        // 5. Create another run token that SHOULD be displayed due to re-enabled tracing
        sender.send(OutputToken::Run { loglines: vec![log_line], symbols: None }).unwrap();
        
        drop(sender);
        
        // Process all tokens through the displayer
        displayer.display(receiver).unwrap();
        
        let output = String::from_utf8(buffer).unwrap();
        
        // Verify the integration worked correctly:
        // 1. Should show control messages
        assert!(output.contains("🔧 trace_logging disabled"), "Missing disable message");
        assert!(output.contains("🔧 trace_logging enabled"), "Missing enable message");
        
        // 2. Should only show ONE trace (the one after re-enabling)
        let trace_count = output.matches("🚀 #0x1000:").count();
        assert_eq!(trace_count, 1, "Expected exactly one trace output after re-enabling, got {}", trace_count);
        
        // 3. Verify the actual trace content is correct
        assert!(output.contains("LDA  #$42"), "Missing expected instruction trace");
        
        println!("Integration test output:\n{}", output);
    }

    #[test]
    fn test_struct_field_offsets() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x2000, "page_header".to_string());
        symbols.add_symbol(0x2100, "_insert_params".to_string());

        // Test various offsets from the base symbols
        let test_cases = [
            // Base address + offset, expected output
            (0x2000, "page_header"),           // Exact match
            (0x2001, "page_header+1"),         // +1 offset
            (0x2010, "page_header+16"),        // +16 offset
            (0x201F, "page_header+31"),        // Maximum allowed offset
            (0x2020, "$2020"),                 // Just beyond max offset
            (0x2100, "_insert_params"),        // Second symbol exact match
            (0x2105, "_insert_params+5"),      // Offset from second symbol
        ];

        for (addr, expected_sym) in test_cases {
            let log_line = LogLine {
                address: 0x1000,
                opcode: 0xad,
                mnemonic: "LDA".to_string(),
                resolution: AddressingModeResolution {
                    target_address: Some(addr),
                    operands: vec![(addr & 0xFF) as u8, (addr >> 8) as u8],
                    addressing_mode: AddressingMode::Absolute([(addr & 0xFF) as u8, (addr >> 8) as u8]),
                },
                outcome: "(0x42)".to_string(),
                cycles: 4,
                registers: RegisterState {
                    accumulator: 0x42,
                    register_x: 0,
                    register_y: 0,
                    status: 0,
                    stack_pointer: 0xFF,
                    command_pointer: 0x1000,
                },
            };

            let formatted = LogLineFormatter::new(&log_line, Some(&symbols)).format();
            assert!(formatted.contains(expected_sym), 
                "Failed for address 0x{:04X}: Expected '{}' in output but got '{}'", 
                addr, expected_sym, formatted);
        }
    }
}
