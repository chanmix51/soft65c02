use std::{fs::File, io::Read, path::PathBuf};

use soft65c02_lib::{execute_step, AddressableIO, LogLine, Memory, Registers};

use crate::{
    until_condition::{Assignment, BooleanExpression, Source, RegisterSource},
    SymbolTable,
    Disassembler,
    AppResult,
    utils,
};

#[derive(Debug, Clone)]
pub enum OutputToken {
    Assertion {
        failure: Option<String>,
        description: String,
    },
    Marker {
        description: String,
    },
    None,
    Run {
        loglines: Vec<LogLine>,
        symbols: Option<SymbolTable>,
    },
    TerminatedRun {
        loglines: Vec<LogLine>,
        symbols: Option<SymbolTable>,
        reason: String,
    },
    Setup(Vec<String>),
    View(Vec<String>),
    ControlAction {
        function: ControllableFunction,
        enabled: bool,
    },
}

pub trait Command {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory, symbols: &mut Option<SymbolTable>) -> AppResult<OutputToken>;
}

// Enum for controllable functions
#[derive(Debug, Clone, PartialEq)]
pub enum ControllableFunction {
    TraceLogging,
}

impl std::fmt::Display for ControllableFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ControllableFunction::TraceLogging => write!(f, "trace_logging"),
        }
    }
}

#[derive(Debug)]
pub enum CliCommand {
    Assert(AssertCommand),
    Marker(String),
    Memory(MemoryCommand),
    None,
    Registers(RegisterCommand),
    Run(RunCommand),
    Disassemble { start: usize, end: usize },
    Enable(ControllableFunction),
    Disable(ControllableFunction),
}

impl Command for CliCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory, symbols: &mut Option<SymbolTable>) -> AppResult<OutputToken> {
        match self {
            Self::Assert(command) => command.execute(registers, memory, symbols),
            Self::Marker(comment) => Ok(OutputToken::Marker {
                description: comment.to_owned(),
            }),
            Self::Memory(command) => command.execute(registers, memory, symbols),
            Self::None => Ok(OutputToken::None),
            Self::Registers(command) => command.execute(registers, memory, symbols),
            Self::Run(command) => command.execute(registers, memory, symbols),
            Self::Disassemble { start, end } => {
                let disassembler = Disassembler::new(memory, symbols);
                let output = disassembler.disassemble_range(*start, *end)?;
                Ok(OutputToken::View(output))
            }
            Self::Enable(function) => Ok(OutputToken::ControlAction { 
                function: function.clone(), 
                enabled: true 
            }),
            Self::Disable(function) => Ok(OutputToken::ControlAction { 
                function: function.clone(), 
                enabled: false 
            }),
        }
    }
}

#[derive(Debug)]
pub struct AssertCommand {
    pub condition: BooleanExpression,
    pub comment: String,
}

impl Command for AssertCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory, _symbols: &mut Option<SymbolTable>) -> AppResult<OutputToken> {
        let token = OutputToken::Assertion {
            failure: self.condition.solve(registers, memory),
            description: self.comment.to_owned(),
        };

        Ok(token)
    }
}

#[derive(Debug)]
pub enum RunAddress {
    Memory(usize),
    InitVector,
}

#[derive(Debug)]
pub struct RunCommand {
    pub stop_condition: BooleanExpression,
    pub continue_condition: BooleanExpression,
    pub start_address: Option<RunAddress>,
}

impl Command for RunCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory, symbols: &mut Option<SymbolTable>) -> AppResult<OutputToken> {
        if let Some(addr) = &self.start_address {
            match addr {
                RunAddress::InitVector => {
                    let lo = memory.read(0xfffc, 1)?[0] as u16;
                    let hi = memory.read(0xfffd, 1)?[0] as u16;
                    registers.command_pointer = ((hi << 8) | lo) as usize;
                }
                RunAddress::Memory(addr) => registers.command_pointer = *addr,
            };
        }

        let mut loglines: Vec<LogLine> = Vec::new();
        let mut cp = registers.command_pointer;

        // Check if we have any cycle limits in the expression
        let has_cycle_limit = self.continue_condition.contains_cycle_limit();
        
        // solve() returns None for truthy conditions (should continue)
        while self.continue_condition.solve(registers, memory).is_none() {
            let line = execute_step(registers, memory)?;
            loglines.push(line);

            let should_stop = self.stop_condition.solve(registers, memory).is_none();
            if registers.command_pointer == cp || should_stop {
                break;
            }
            cp = registers.command_pointer;
        }

        // After stopping, check if we hit any cycle limits
        if has_cycle_limit && self.continue_condition.was_cycle_limit_hit(registers, memory) {
            Ok(OutputToken::TerminatedRun {
                loglines,
                symbols: symbols.clone(),
                reason: "Cycle count limit exceeded".to_string(),
            })
        } else {
            Ok(OutputToken::Run { 
                loglines,
                symbols: symbols.clone(),
            })
        }
    }
}

impl BooleanExpression {
    fn contains_cycle_limit(&self) -> bool {
        match self {
            Self::StrictlyLesser(left, _) | Self::StrictlyGreater(left, _) |
            Self::LesserOrEqual(left, _) | Self::GreaterOrEqual(left, _) |
            Self::Equal(left, _) | Self::Different(left, _) => {
                matches!(left, Source::Register(RegisterSource::CycleCount))
            }
            Self::And(left, right) | Self::Or(left, right) => {
                left.contains_cycle_limit() || right.contains_cycle_limit()
            }
            Self::Not(expr) => expr.contains_cycle_limit(),
            Self::Value(_) => false,
            Self::MemorySequence(_, _) => false,
        }
    }

    fn was_cycle_limit_hit(&self, registers: &Registers, memory: &Memory) -> bool {
        match self {
            // For cycle limits, we want to detect when the condition becomes false
            // This means we've hit the limit
            Self::StrictlyLesser(left, right) => {
                if matches!(left, Source::Register(RegisterSource::CycleCount)) {
                    if let Source::Value(limit) = right {
                        return registers.cycle_count >= *limit as u64;
                    }
                }
                false
            }
            Self::StrictlyGreater(left, right) => {
                if matches!(left, Source::Register(RegisterSource::CycleCount)) {
                    if let Source::Value(limit) = right {
                        return registers.cycle_count <= *limit as u64;
                    }
                }
                false
            }
            Self::LesserOrEqual(left, right) => {
                if matches!(left, Source::Register(RegisterSource::CycleCount)) {
                    if let Source::Value(limit) = right {
                        return registers.cycle_count > *limit as u64;
                    }
                }
                false
            }
            Self::GreaterOrEqual(left, right) => {
                if matches!(left, Source::Register(RegisterSource::CycleCount)) {
                    if let Source::Value(limit) = right {
                        return registers.cycle_count < *limit as u64;
                    }
                }
                false
            }
            Self::Equal(left, right) => {
                if matches!(left, Source::Register(RegisterSource::CycleCount)) {
                    if let Source::Value(limit) = right {
                        // For equality, we've hit the limit if we're no longer equal
                        return registers.cycle_count != *limit as u64;
                    }
                }
                false
            }
            Self::Different(left, right) => {
                if matches!(left, Source::Register(RegisterSource::CycleCount)) {
                    if let Source::Value(limit) = right {
                        // For inequality, we've hit the limit if we become equal
                        return registers.cycle_count == *limit as u64;
                    }
                }
                false
            }
            Self::And(left, right) => {
                left.was_cycle_limit_hit(registers, memory) || right.was_cycle_limit_hit(registers, memory)
            }
            Self::Or(left, right) => {
                left.was_cycle_limit_hit(registers, memory) || right.was_cycle_limit_hit(registers, memory)
            }
            Self::Not(expr) => expr.was_cycle_limit_hit(registers, memory),
            _ => false
        }
    }
}

#[derive(Debug)]
pub enum RegisterCommand {
    Flush,
    Set { assignment: Assignment },
    Show { register: Option<RegisterSource> },
}

impl Command for RegisterCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory, _symbols: &mut Option<SymbolTable>) -> AppResult<OutputToken> {
        let outputs = match self {
            Self::Flush => {
                registers.initialize(0x0000);

                vec!["registers flushed".to_string()]
            }
            Self::Set { assignment } => assignment.execute(registers, memory)?,
            Self::Show { register } => {
                match register {
                    Some(reg) => {
                        let value = reg.get_value(registers);
                        match reg {
                            RegisterSource::Accumulator => vec![format!("A = 0x{:02X}  ({})", value, value)],
                            RegisterSource::RegisterX => vec![format!("X = 0x{:02X}  ({})", value, value)],
                            RegisterSource::RegisterY => vec![format!("Y = 0x{:02X}  ({})", value, value)],
                            RegisterSource::Status => vec![format!("S = 0b{:08b}  {}", value, registers.format_status())],
                            RegisterSource::StackPointer => vec![format!("SP = 0x{:02X}  ({})", value, value)],
                            RegisterSource::CommandPointer => vec![format!("CP = 0x{:04X}", value)],
                            RegisterSource::CycleCount => vec![format!("cycle_count = {}", value)],
                        }
                    }
                    None => {
                        vec![
                            "Registers:".to_string(),
                            format!("   A  = 0x{:02X}  ({})", registers.accumulator, registers.accumulator),
                            format!("   X  = 0x{:02X}  ({})", registers.register_x, registers.register_x),
                            format!("   Y  = 0x{:02X}  ({})", registers.register_y, registers.register_y),
                            format!("   S  = 0b{:08b}  {}", registers.get_status_register(), registers.format_status()),
                            format!("   SP = 0x{:02X}  ({})", registers.stack_pointer, registers.stack_pointer),
                            format!("   CP = 0x{:04X}", registers.command_pointer),
                            format!("   cycle_count = {}", registers.cycle_count),
                        ]
                    }
                }
            }
        };

        let token = OutputToken::Setup(outputs);

        Ok(token)
    }
}

#[derive(Debug)]
pub struct MemorySegment {
    pub address: usize,
    pub data: Vec<u8>,
}

#[derive(Debug)]
pub enum MemoryCommand {
    Flush,
    Load { address: usize, filepath: PathBuf },
    Write { address: usize, bytes: Vec<u8> },
    Fill { start: usize, end: usize, value: u8 },
    LoadSegments { segments: Vec<MemorySegment> },
    LoadSymbols { symbols: SymbolTable },
    AddSymbol { name: String, value: u16 },
    RemoveSymbol { name: String },
    Show { address: usize, length: usize, width: Option<usize>, description: Option<String> },
}

impl Command for MemoryCommand {
    fn execute(&self, _registers: &mut Registers, memory: &mut Memory, symbols: &mut Option<SymbolTable>) -> AppResult<OutputToken> {
        let output = match self {
            Self::Flush => {
                *memory = Memory::new_with_ram();
                Vec::new()
            }
            Self::Write { address, bytes } => match bytes.len() {
                0 => vec!["nothing was written".to_string()],
                1 => {
                    memory.write(*address, bytes)?;
                    vec!["1 byte written".to_string()]
                }
                n => {
                    memory.write(*address, bytes)?;
                    vec![format!("{n} bytes written")]
                }
            },
            Self::Fill { start, end, value } => {
                // Calculate length without wrapping, stopping at boundaries
                let len = if end >= start {
                    // Going up: clamp end to 0xFFFF
                    let clamped_end = *end.min(&0xFFFF);
                    clamped_end - *start + 1
                } else {
                    // Going down: clamp start to 0x0000
                    let clamped_start = *start.max(end);
                    *start - clamped_start + 1
                };

                // Write the bytes in chunks to avoid large allocations
                let chunk_size = 1024;
                let mut remaining = len;
                let mut current_addr = if end >= start { *start } else { *end };

                while remaining > 0 {
                    let write_size = remaining.min(chunk_size);
                    let chunk = vec![*value; write_size];
                    memory.write(current_addr, &chunk)?;
                    current_addr = current_addr.wrapping_add(write_size);
                    remaining -= write_size;
                }

                vec![format!("{} bytes filled with 0x{:02X}", len, value)]
            },
            Self::Load { address, filepath } => {
                let vec = {
                    let mut f = File::open(filepath)?;
                    let mut buffer: Vec<u8> = vec![];
                    f.read_to_end(&mut buffer)?;

                    buffer
                };
                let buffer = vec;
                memory.write(*address, &buffer).unwrap();

                vec![format!(
                    "{} bytes loaded from '{}' at #0x{address:04X}.",
                    buffer.len(),
                    filepath.display()
                )]
            }
            Self::LoadSegments { segments } => {
                for segment in segments {
                    memory.write(segment.address, &segment.data)?;
                }
                vec![format!("{} segments loaded.", segments.len())]
            }
            Self::LoadSymbols { symbols: new_symbols } => {
                let count = new_symbols.len();
                *symbols = Some(new_symbols.clone());
                vec![format!("{} symbols loaded", count)]
            }
            Self::AddSymbol { name, value } => {
                // Initialize symbol table if it doesn't exist
                if symbols.is_none() {
                    *symbols = Some(SymbolTable::new());
                }
                // This unwrap is now safe since we ensure symbols exists above
                let symtable = symbols.as_mut().unwrap();
                symtable.add_symbol(*value, name.clone());
                vec![format!("Symbol {} added with value 0x{:04X}", name, value)]
            }
            Self::RemoveSymbol { name } => {
                if let Some(symtable) = symbols {
                    if let Some(addr) = symtable.get_address(name) {
                        symtable.remove_symbol(name);
                        vec![format!("Symbol {} (was 0x{:04X}) removed", name, addr)]
                    } else {
                        vec![format!("Symbol {} not found", name)]
                    }
                } else {
                    vec!["No symbol table available".to_string()]
                }
            },
            Self::Show { address, length, width, description } => {
                let data = memory.read(*address, *length)?;
                let mut output = Vec::new();
                if let Some(desc) = description {
                    output.push(desc.clone());
                }
                let display_width = width.unwrap_or(16);
                output.push(format!("\n{}", utils::format_hex_dump_with_width(*address, &data, display_width)));
                output
            }
        };

        Ok(OutputToken::Setup(output))
    }
}

#[cfg(test)]
mod assert_command_tests {
    use super::*;

    #[test]
    fn test_assert_command_ok() {
        let command = AssertCommand {
            condition: BooleanExpression::Value(true),
            comment: "nice comment".to_string(),
        };
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(
            matches!(token, OutputToken::Assertion { failure, description } if failure.is_none() && description == *"nice comment")
        );
    }

    #[test]
    fn test_assert_command_fails() {
        let command = AssertCommand {
            condition: BooleanExpression::Value(false),
            comment: "failing assertion".to_string(),
        };
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();

        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(
            matches!(token, OutputToken::Assertion { failure, description } if failure.is_some() && description == *"failing assertion")
        );
    }
}

#[cfg(test)]
mod run_command_tests {
    use soft65c02_lib::AddressableIO;

    use crate::until_condition::{RegisterSource, Source};

    use super::*;

    #[test]
    fn simple_run() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(true),
            continue_condition: BooleanExpression::Value(true),
            start_address: None,
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa9, 0xc0]).unwrap(); // LDA #0xc0
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Run { loglines, symbols } if loglines.len() == 1 && symbols.is_none()));
    }

    #[test]
    fn run_from_addr() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(true),
            continue_condition: BooleanExpression::Value(true),
            start_address: Some(RunAddress::Memory(0x1234)),
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1234, &[0xa9, 0xc0]).unwrap(); // LDA #0xc0
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Run { loglines, symbols } if loglines.len() == 1 && symbols.is_none()));
    }

    #[test]
    fn run_init_vector() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(true),
            continue_condition: BooleanExpression::Value(true),
            start_address: Some(RunAddress::InitVector),
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        memory.write(0xfffc, &[0x34, 0x12]).unwrap(); // init vector
        memory.write(0x1234, &[0xa9, 0xc0]).unwrap(); // LDA #0xc0
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Run { loglines, symbols } if loglines.len() == 1 && symbols.is_none()));
        assert_eq!(0x1236, registers.command_pointer);
        assert_eq!(0xc0, registers.accumulator);
    }

    #[test]
    fn run_with_condition() {
        let command = RunCommand {
            stop_condition: BooleanExpression::StrictlyGreater(
                Source::Register(RegisterSource::RegisterX),
                Source::Value(0),
            ),
            continue_condition: BooleanExpression::Value(true),
            start_address: Some(RunAddress::Memory(0x1234)),
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1234, &[0xa9, 0xc0, 0xaa]).unwrap(); // LDA #0xc0; TXA
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Run { loglines, symbols } if loglines.len() == 2 && symbols.is_none()));
    }

    #[test]
    fn run_stops_on_loop() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::Value(true),
            start_address: None,
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xd0, 0b11111110]).unwrap(); // BNE -1
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Run { loglines, symbols } if loglines.len() == 1 && symbols.is_none()));
    }

    #[test]
    fn test_while_condition_checked_before_execution() {
        let command = RunCommand {
            // For while conditions, we use continue_condition for the check
            continue_condition: BooleanExpression::Equal(
                Source::Register(RegisterSource::RegisterX),
                Source::Value(0),
            ),
            // Stop condition is false so it only stops on continue check or infinite loop
            stop_condition: BooleanExpression::Value(false),
            start_address: Some(RunAddress::Memory(0x1234)),
        };
        let mut registers = Registers::new_initialized(0x1234);
        registers.register_x = 1; // Set X to 1 so the condition is false immediately
        let mut memory = Memory::new_with_ram();
        memory.write(0x1234, &[0xe8]).unwrap(); // INX - increment X
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // If the condition is checked before execution, no instructions should be executed
        assert!(matches!(token, OutputToken::Run { loglines, symbols } if loglines.is_empty() && symbols.is_none()));
        // X should still be 1 since the INX instruction should not have executed
        assert_eq!(registers.register_x, 1);
    }

    #[test]
    fn test_while_cycle_count_condition() {
        let command = RunCommand {
            // Continue while cycle_count < 0x100
            continue_condition: BooleanExpression::StrictlyLesser(
                Source::Register(RegisterSource::CycleCount),
                Source::Value(0x100),
            ),
            stop_condition: BooleanExpression::Value(false),
            start_address: Some(RunAddress::Memory(0x1000)),
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        
        // Program:
        // 1000: LDX #$64    ; Load X with 100 (decimal) - 2 cycles
        // 1002: STX $80     ; Store X to memory - 3 cycles
        // 1004: DEX         ; Decrement X - 2 cycles
        // 1005: BNE $1002   ; Branch if not zero (to STX) - 3 cycles
        // Total per iteration: 8 cycles
        // Initial LDX: 2 cycles
        // Will stop when cycles >= 256, might overshoot due to multi-cycle instructions
        memory.write(0x1000, &[0xa2, 0x64]).unwrap();     // LDX #$64
        memory.write(0x1002, &[0x86, 0x80]).unwrap();     // STX $80
        memory.write(0x1004, &[0xca]).unwrap();           // DEX
        memory.write(0x1005, &[0xd0, 0xfb]).unwrap();     // BNE $1002 (-5 bytes)
        memory.write(0x1007, &[0xdb]).unwrap();           // STP

        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Read the last value stored at $80
        let final_x = memory.read(0x80, 1).unwrap()[0];

        // After 31 complete iterations plus partial:
        // - Initial X = 100 (0x64)
        // - Decremented 31 times to 0x45
        // - the final executions look like:
        // Cycles: 2 (total: 255) - LogLine { address: 4100, opcode: 202, mnemonic: "DEX", resolution: AddressingModeResolution { operands: [], addressing_mode: Implied, target_address: None }, outcome: "[X=0x44][S=nv-Bdizc]", cycles: 2 }
        // Cycles: 3 (total: 258) - LogLine { address: 4101, opcode: 208, mnemonic: "BNE", resolution: AddressingModeResolution { operands: [251], addressing_mode: Relative(4101, [251]), target_address: None }, outcome: "[CP=0x1002]", cycles: 3 }
        // thus we have a DEX that doesn't store to $80, and so it is 1 less than the memory value.
        // Because executions have to complete, the cycle count will often jump over the limit before the trigger condition is checked, hence 258 is larger than 256, but was the first after going over 256

        assert_eq!(final_x, 0x45, "X should be 0x45 (69) after 31 iterations plus one more store");
        assert_eq!(registers.register_x, 0x44, "X register should be 0x44 (68) after final DEX");

        // Verify we got a TerminatedRun token with the cycle limit message
        match result {
            OutputToken::TerminatedRun { loglines: _, symbols: _, reason } => {
                assert_eq!(reason, "Cycle count limit exceeded");
            }
            _ => panic!("Expected TerminatedRun token"),
        }
        assert!(registers.cycle_count >= 0x100, "Should have executed at least 256 cycles");
    }

    #[test]
    fn test_run_with_cycle_limit() {
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        
        // Program that just decrements X until zero
        // DEX (2 cycles) followed by BNE (-2) (3 cycles when taken)
        // So each loop is 5 cycles
        memory.write(0x1000, &[0xca, 0xd0, 0xfd]).unwrap(); // DEX, BNE -2
        registers.register_x = 10; // Will take 50 cycles to complete

        // This should succeed - needs exactly 50 cycles
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::And(
                Box::new(BooleanExpression::Different(
                    Source::Register(RegisterSource::RegisterX),
                    Source::Value(0),
                )),
                Box::new(BooleanExpression::StrictlyLesser(
                    Source::Register(RegisterSource::CycleCount),
                    Source::Value(60), // Give it some headroom
                )),
            ),
            start_address: None,
        };
        
        // This should complete normally
        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        match result {
            OutputToken::Run { .. } => (),
            _ => panic!("Expected normal Run token"),
        }
        assert_eq!(registers.register_x, 0);

        // Reset for next test
        registers = Registers::new_initialized(0x1000);
        registers.register_x = 100; // Would take 500 cycles to complete

        // This should fail - cycle limit too low
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::And(
                Box::new(BooleanExpression::Different(
                    Source::Register(RegisterSource::RegisterX),
                    Source::Value(0),
                )),
                Box::new(BooleanExpression::StrictlyLesser(
                    Source::Register(RegisterSource::CycleCount),
                    Source::Value(20),
                )),
            ),
            start_address: None,
        };
        
        // This should return a TerminatedRun token
        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        match result {
            OutputToken::TerminatedRun { loglines: _, symbols: _, reason } => {
                assert_eq!(reason, "Cycle count limit exceeded");
            }
            _ => panic!("Expected TerminatedRun token"),
        }
        assert!(registers.register_x > 0); // Should not have completed

        // Test that non-cycle conditions don't cause termination
        registers = Registers::new_initialized(0x1000);
        registers.register_x = 5;

        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::Different(
                Source::Register(RegisterSource::RegisterX),
                Source::Value(3), // Stop when X reaches 3
            ),
            start_address: None,
        };
        
        // This should complete normally when X reaches 3
        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        match result {
            OutputToken::Run { .. } => (),
            _ => panic!("Expected normal Run token"),
        }
        assert_eq!(registers.register_x, 3);
    }

    #[test]
    fn test_run_with_complex_cycle_limits_first_part() {
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        
        memory.write(0x1000, &[0xca, 0xc8, 0xd0, 0xfc]).unwrap();
        
        // Initial state: X=50, Y=0 - Much higher X means we'll hit cycle limit before X ≤ 10
        registers.register_x = 50;
        registers.register_y = 0;

        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::Or(
                Box::new(BooleanExpression::And(
                    Box::new(BooleanExpression::StrictlyGreater(
                        Source::Register(RegisterSource::RegisterX),
                        Source::Value(10),
                    )),
                    Box::new(BooleanExpression::StrictlyLesser(
                        Source::Register(RegisterSource::CycleCount),
                        Source::Value(100),
                    )),
                )),
                Box::new(BooleanExpression::And(
                    Box::new(BooleanExpression::LesserOrEqual(
                        Source::Register(RegisterSource::RegisterY),
                        Source::Value(5),
                    )),
                    Box::new(BooleanExpression::StrictlyLesser(
                        Source::Register(RegisterSource::CycleCount),
                        Source::Value(200),
                    )),
                )),
            ),
            start_address: None,
        };

        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        
        // Should fail due to hitting the 100 cycle limit while X > 10
        match result {
            OutputToken::TerminatedRun { loglines: _, symbols: _, reason } => {
                assert_eq!(reason, "Cycle count limit exceeded");
            }
            _ => panic!("Expected TerminatedRun token"),
        }
        assert!(registers.cycle_count >= 100); // Should stop after 100 cycles
        assert!(registers.register_x > 10); // And X should still be > 10
    }

    #[test]
    fn test_run_with_complex_cycle_limits_second_part() {
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        
        memory.write(0x1000, &[0xca, 0xc8, 0xd0, 0xfc]).unwrap();
        
        // Initial state: X=9, Y=0
        // Both conditions must be true to continue:
        // 1. X != 11 AND cycles < 500 (won't be the limiting factor)
        // 2. Y <= 5 AND cycles < 10 (will hit cycle limit first)
        registers.register_x = 9;
        registers.register_y = 0;

        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::And(
                Box::new(BooleanExpression::And(
                    Box::new(BooleanExpression::Different(
                        Source::Register(RegisterSource::RegisterX),
                        Source::Value(11),
                    )),
                    Box::new(BooleanExpression::StrictlyLesser(
                        Source::Register(RegisterSource::CycleCount),
                        Source::Value(500),
                    )),
                )),
                Box::new(BooleanExpression::And(
                    Box::new(BooleanExpression::LesserOrEqual(
                        Source::Register(RegisterSource::RegisterY),
                        Source::Value(5),
                    )),
                    Box::new(BooleanExpression::StrictlyLesser(
                        Source::Register(RegisterSource::CycleCount),
                        Source::Value(10),
                    )),
                )),
            ),
            start_address: None,
        };

        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        
        // Should fail due to hitting the 10 cycle limit while Y is still ≤ 5
        match result {
            OutputToken::TerminatedRun { loglines: _, symbols: _, reason } => {
                assert_eq!(reason, "Cycle count limit exceeded");
            }
            _ => panic!("Expected TerminatedRun token"),
        }
        assert!(registers.register_y <= 5); // Y should still be small
        assert!(registers.cycle_count >= 10); // Should be 10 or more
        assert_ne!(registers.register_x, 11); // X should not have reached 11
    }

    #[test]
    fn test_run_until_cycle_limit() {
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        
        // Program that just decrements X until zero
        // DEX (2 cycles) followed by BNE (-2) (3 cycles when taken)
        // So each loop is 5 cycles
        memory.write(0x1000, &[0xca, 0xd0, 0xfd]).unwrap(); // DEX, BNE -2
        registers.register_x = 10; // Will take 50 cycles to complete

        // Run until cycles > 20 (should stop after hitting 21+ cycles)
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            continue_condition: BooleanExpression::Not(Box::new(BooleanExpression::StrictlyGreater(
                Source::Register(RegisterSource::CycleCount),
                Source::Value(20),
            ))),
            start_address: None,
        };
        
        let result = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        
        // Should complete normally since we hit the target cycle count
        match result {
            OutputToken::Run { loglines, .. } => {
                assert!(registers.cycle_count > 20, "Should have executed more than 20 cycles");
                assert!(registers.register_x > 0, "Should not have completed the full loop");
                assert!(!loglines.is_empty(), "Should have executed some instructions");
            }
            _ => panic!("Expected Run token"),
        }
    }
}

#[cfg(test)]
mod register_command_tests {
    use crate::until_condition::{RegisterSource, Source};

    use super::*;

    #[test]
    fn test_flush() {
        let command = RegisterCommand::Flush;
        let mut registers = Registers::new_initialized(0xffff);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"registers flushed"));
        assert_eq!(0x0000, registers.command_pointer);
    }

    #[test]
    fn test_set() {
        let command = RegisterCommand::Set {
            assignment: Assignment::new(Source::Value(0xff), RegisterSource::RegisterX),
        };
        let mut registers = Registers::new_initialized(0xffff);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"register X set to 0xff"));
        assert_eq!(0xff, registers.register_x);
    }

    #[test]
    fn test_show() {
        let command = RegisterCommand::Show { register: None };
        let mut registers = Registers::new_initialized(0x1234);
        registers.accumulator = 0x42;
        registers.register_x = 0x10;
        registers.register_y = 0x20;
        registers.stack_pointer = 0xFE;
        registers.cycle_count = 1847;
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        if let OutputToken::Setup(lines) = token {
            assert_eq!(lines[0], "Registers:");
            assert_eq!(lines[1], "   A  = 0x42  (66)");
            assert_eq!(lines[2], "   X  = 0x10  (16)");
            assert_eq!(lines[3], "   Y  = 0x20  (32)");
            assert!(lines[4].starts_with("   S  = 0b"));
            assert_eq!(lines[5], "   SP = 0xFE  (254)");
            assert_eq!(lines[6], "   CP = 0x1234");
            assert_eq!(lines[7], "   cycle_count = 1847");
        } else {
            panic!("Expected Setup token");
        }
    }

    #[test]
    fn test_show_cycle_count() {
        let command = RegisterCommand::Show { register: Some(RegisterSource::CycleCount) };
        let mut registers = Registers::new_initialized(0x1234);
        registers.cycle_count = 1847;
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        if let OutputToken::Setup(lines) = token {
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0], "cycle_count = 1847");
        } else {
            panic!("Expected Setup token");
        }
    }

    #[test]
    fn test_show_accumulator() {
        let command = RegisterCommand::Show { register: Some(RegisterSource::Accumulator) };
        let mut registers = Registers::new_initialized(0x1234);
        registers.accumulator = 0x42;
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        if let OutputToken::Setup(lines) = token {
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0], "A = 0x42  (66)");
        } else {
            panic!("Expected Setup token");
        }
    }
}

#[cfg(test)]
mod memory_command_tests {
    use soft65c02_lib::AddressableIO;

    use super::*;

    #[test]
    fn test_flush_command() {
        let command = MemoryCommand::Flush;
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x0000, &[0x01, 0x02, 0x03]).unwrap();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert_eq!(vec![0x00, 0x00, 0x00], memory.read(0x000, 3).unwrap());
        assert!(matches!(token, OutputToken::Setup(s) if s.is_empty()));
    }

    #[test]
    fn test_write_command() {
        let command = MemoryCommand::Write {
            address: 0x1000,
            bytes: vec![0x01, 0x02, 0x03],
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Setup(v) if v[0] == *"3 bytes written"));
        assert_eq!(
            &[0x01, 0x02, 0x03],
            memory.read(0x1000, 3).unwrap().as_slice()
        );
    }

    #[test]
    fn test_write_no_byte() {
        let command = MemoryCommand::Write {
            address: 0x1000,
            bytes: Vec::new(),
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"nothing was written"));
        assert_eq!(
            &[0x00, 0x00, 0x00],
            memory.read(0x1000, 3).unwrap().as_slice()
        );
    }

    #[test]
    fn test_write_one_byte() {
        let command = MemoryCommand::Write {
            address: 0x1000,
            bytes: vec![0x01],
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"1 byte written"));
        assert_eq!(
            &[0x01, 0x00, 0x00],
            memory.read(0x1000, 3).unwrap().as_slice()
        );
    }

    #[test]
    fn test_load() {
        let filepath = PathBuf::new().join("../Cargo.toml");
        let command = MemoryCommand::Load {
            address: 0x1000,
            filepath,
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        let expected = "bytes loaded from '../Cargo.toml' at #0x1000.".to_owned();
        assert!(matches!(token, OutputToken::Setup(s) if s[0].contains(&expected)));
    }

    #[test]
    fn test_load_segments_empty() {
        let command = MemoryCommand::LoadSegments { 
            segments: Vec::new() 
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"0 segments loaded."));
    }

    #[test]
    fn test_load_segments_single() {
        let command = MemoryCommand::LoadSegments { 
            segments: vec![
                MemorySegment {
                    address: 0x2000,
                    data: vec![0x01, 0x02, 0x03],
                }
            ] 
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Verify the output token
        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"1 segments loaded."));
        
        // Verify the memory contents
        assert_eq!(
            &[0x01, 0x02, 0x03],
            memory.read(0x2000, 3).unwrap().as_slice()
        );
    }

    #[test]
    fn test_load_segments_multiple() {
        let command = MemoryCommand::LoadSegments { 
            segments: vec![
                MemorySegment {
                    address: 0x2000,
                    data: vec![0x01, 0x02, 0x03],
                },
                MemorySegment {
                    address: 0x3000,
                    data: vec![0xFF, 0xFE],
                },
                MemorySegment {
                    address: 0x02E0,
                    data: vec![0x34, 0x12],
                }
            ] 
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Verify the output token
        assert!(matches!(token, OutputToken::Setup(s) if s[0] == *"3 segments loaded."));
        
        // Verify each segment was written correctly
        assert_eq!(
            &[0x01, 0x02, 0x03],
            memory.read(0x2000, 3).unwrap().as_slice()
        );
        assert_eq!(
            &[0xFF, 0xFE],
            memory.read(0x3000, 2).unwrap().as_slice()
        );
        assert_eq!(
            &[0x34, 0x12],
            memory.read(0x02E0, 2).unwrap().as_slice()
        );
    }

    #[test]
    fn test_memory_fill_execution() {
        let mut memory = Memory::new_with_ram();
        let mut registers = Registers::new(0);
        let mut symbols = None;

        // Fill a range with a value
        let command = MemoryCommand::Fill {
            start: 0x1000,
            end: 0x1002,
            value: 0x42,
        };
        let result = command.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        
        // Check the output message
        assert!(matches!(result, OutputToken::Setup(msgs) if msgs[0] == "3 bytes filled with 0x42"));
        
        // Verify the memory contents
        assert_eq!(memory.read(0x1000, 1).unwrap()[0], 0x42);
        assert_eq!(memory.read(0x1001, 1).unwrap()[0], 0x42);
        assert_eq!(memory.read(0x1002, 1).unwrap()[0], 0x42);
        
        // Check that memory outside the range is unaffected
        assert_eq!(memory.read(0x0FFF, 1).unwrap()[0], 0x00);
        assert_eq!(memory.read(0x1003, 1).unwrap()[0], 0x00);
    }

    #[test]
    fn test_memory_show_output() {
        let command = MemoryCommand::Show {
            address: 0x1000,
            length: 16,
            width: None,
            description: None,
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        // Write test data with a mix of hex values and ASCII printable characters
        // "Hello, World!" followed by some non-printable bytes
        memory.write(0x1000, &[
            b'H', b'e', b'l', b'l', b'o', b',', b' ', b'W',  // Hello, W
            b'o', b'r', b'l', b'd', b'!', 0x00, 0x01, 0xFF   // orld!...
        ]).unwrap();

        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Check that we got a Setup token with the expected hex dump
        match token {
            OutputToken::Setup(lines) => {
                assert_eq!(lines.len(), 1);
                let expected = format!("\n1000 : 48 65 6C 6C 6F 2C 20 57 6F 72 6C 64 21 00 01 FF | Hello, World!...");
                assert_eq!(lines[0], expected);
            }
            _ => panic!("Expected Setup token"),
        }

        // Test with description
        let command = MemoryCommand::Show {
            address: 0x1000,
            length: 16,
            width: None,
            description: Some("Showing Hello World".to_string()),
        };
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        match token {
            OutputToken::Setup(lines) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(lines[0], "Showing Hello World");
                let expected = format!("\n1000 : 48 65 6C 6C 6F 2C 20 57 6F 72 6C 64 21 00 01 FF | Hello, World!...");
                assert_eq!(lines[1], expected);
            }
            _ => panic!("Expected Setup token"),
        }
    }

    #[test]
    fn test_memory_show_output_partial_line() {
        let command = MemoryCommand::Show {
            address: 0x1000,
            length: 7,  // Just show "Hello, "
            width: None,
            description: None,
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        // Write test data with printable ASCII characters
        memory.write(0x1000, &[
            b'H', b'e', b'l', b'l', b'o', b',', b' ',  // "Hello, "
        ]).unwrap();

        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Check that we got a Setup token with the expected hex dump
        match token {
            OutputToken::Setup(lines) => {
                assert_eq!(lines.len(), 1);
                let expected = format!("\n1000 : 48 65 6C 6C 6F 2C 20                            | Hello, ");
                assert_eq!(lines[0], expected);
            }
            _ => panic!("Expected Setup token"),
        }
    }

    #[test]
    fn test_memory_show_output_multiple_lines() {
        let command = MemoryCommand::Show {
            address: 0x1000,
            length: 32,  // Two full lines
            width: None,
            description: None,
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        // Write test data with printable ASCII characters
        memory.write(0x1000, &[
            // First line: "Hello, World!" + 3 special chars
            b'H', b'e', b'l', b'l', b'o', b',', b' ', b'W',
            b'o', b'r', b'l', b'd', b'!', 0x00, 0x01, 0xFF,
            // Second line: "Testing 123" + some control chars
            b'T', b'e', b's', b't', b'i', b'n', b'g', b' ',
            b'1', b'2', b'3', 0x07, 0x08, 0x09, 0x0A, 0x0B
        ]).unwrap();

        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Check that we got a Setup token with the expected hex dump
        match token {
            OutputToken::Setup(lines) => {
                assert_eq!(lines.len(), 1);
                let expected = format!("\n1000 : 48 65 6C 6C 6F 2C 20 57 6F 72 6C 64 21 00 01 FF | Hello, World!...\n1010 : 54 65 73 74 69 6E 67 20 31 32 33 07 08 09 0A 0B | Testing 123.....");
                assert_eq!(lines[0], expected);
            }
            _ => panic!("Expected Setup token"),
        }
    }

    #[test]
    fn test_memory_show_output_with_custom_width() {
        let command = MemoryCommand::Show {
            address: 0x1000,
            length: 16,  // 16 bytes total
            width: Some(8),  // 8 bytes per line instead of default 16
            description: None,
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        // Write test data with printable ASCII characters
        memory.write(0x1000, &[
            b'H', b'e', b'l', b'l', b'o', b',', b' ', b'W',  // Hello, W
            b'o', b'r', b'l', b'd', b'!', 0x00, 0x01, 0xFF   // orld!...
        ]).unwrap();

        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Check that we got a Setup token with the expected hex dump (8 bytes per line)
        match token {
            OutputToken::Setup(lines) => {
                assert_eq!(lines.len(), 1);
                let expected = format!("\n1000 : 48 65 6C 6C 6F 2C 20 57 | Hello, W\n1008 : 6F 72 6C 64 21 00 01 FF | orld!...");
                assert_eq!(lines[0], expected);
            }
            _ => panic!("Expected Setup token"),
        }
    }

    #[test]
    fn test_memory_show_output_with_narrow_width() {
        let command = MemoryCommand::Show {
            address: 0x2000,
            length: 12,  // 12 bytes total
            width: Some(4),  // 4 bytes per line
            description: Some("Narrow display".to_string()),
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        // Write test data
        memory.write(0x2000, &[
            0x01, 0x02, 0x03, 0x04,  // Line 1
            0x05, 0x06, 0x07, 0x08,  // Line 2
            0x09, 0x0A, 0x0B, 0x0C   // Line 3
        ]).unwrap();

        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();

        // Check that we got a Setup token with the expected hex dump (4 bytes per line)
        match token {
            OutputToken::Setup(lines) => {
                assert_eq!(lines.len(), 2);
                assert_eq!(lines[0], "Narrow display");
                let expected = format!("\n2000 : 01 02 03 04 | ....\n2004 : 05 06 07 08 | ....\n2008 : 09 0A 0B 0C | ....");
                assert_eq!(lines[1], expected);
            }
            _ => panic!("Expected Setup token"),
        }
    }

    #[test]
    fn test_add_symbol_with_no_existing_table() {
        let command = MemoryCommand::AddSymbol { 
            name: "test_symbol".to_string(), 
            value: 0x1234 
        };
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();
        let mut symbols = None; // Start with no symbol table
        
        let token = command.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        
        // Verify the symbol table was created and symbol added
        assert!(symbols.is_some());
        let symbol_table = symbols.as_ref().unwrap();
        assert_eq!(symbol_table.get_address("test_symbol"), Some(0x1234));
        
        // Verify the output message
        assert!(matches!(token, OutputToken::Setup(ref lines) if lines.len() == 1));
        if let OutputToken::Setup(lines) = token {
            assert!(lines[0].contains("Symbol test_symbol added with value 0x1234"));
        }
    }
}

#[cfg(test)]
mod control_command_tests {
    use super::*;

    #[test]
    fn test_enable_command_execution() {
        let command = CliCommand::Enable(ControllableFunction::TraceLogging);
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();
        let mut symbols = None;
        
        let result = command.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        
        assert!(matches!(result, OutputToken::ControlAction { 
            function: ControllableFunction::TraceLogging, 
            enabled: true 
        }));
    }

    #[test]
    fn test_disable_command_execution() {
        let command = CliCommand::Disable(ControllableFunction::TraceLogging);
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();
        let mut symbols = None;
        
        let result = command.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        
        assert!(matches!(result, OutputToken::ControlAction { 
            function: ControllableFunction::TraceLogging, 
            enabled: false 
        }));
    }

    #[test]
    fn test_controllable_function_display() {
        let function = ControllableFunction::TraceLogging;
        assert_eq!(format!("{}", function), "trace_logging");
    }
}

#[cfg(test)]
mod disassemble_command_tests {
    use super::*;

    #[test]
    fn test_basic_disassemble() {
        let command = CliCommand::Disassemble { 
            start: 0x1000, 
            end: 0x1000  // Length of 1 byte
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        // Write a simple instruction (LDA #$42)
        memory.write(0x1000, &[0xa9, 0x42]).unwrap();
        
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        
        // Check that we got a View token with the expected disassembly
        match token {
            OutputToken::View(lines) => {
                assert!(lines.len() > 2); // At least header, one instruction, and footer
                assert!(lines[0].contains("Start of disassembly"));
                assert!(lines[1].contains("LDA  #$42")); // Check the instruction
                assert!(lines[2].contains("End of disassembly"));
            }
            _ => panic!("Expected OutputToken::View"),
        }
    }

    #[test]
    fn test_disassemble_with_symbols() {
        let command = CliCommand::Disassemble { 
            start: 0x1000, 
            end: 0x1001  // Length of 2 bytes
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let mut symbols = Some(SymbolTable::new());
        
        // Add a test symbol
        if let Some(symtab) = &mut symbols {
            symtab.add_symbol(0x1000, "start".to_string());
        }
        
        // Write a simple instruction (LDA #$42)
        memory.write(0x1000, &[0xa9, 0x42]).unwrap();
        
        let token = command.execute(&mut registers, &mut memory, &mut symbols).unwrap();
        
        // Check that we got a View token with the symbol in the output
        match token {
            OutputToken::View(lines) => {
                assert!(lines.len() > 2);
                assert!(lines[1].contains("start")); // Symbol should appear
                assert!(lines[2].contains("LDA  #$42")); // Check the instruction
            }
            _ => panic!("Expected OutputToken::View"),
        }
    }

    #[test]
    fn test_disassemble_empty_range() {
        let command = CliCommand::Disassemble { 
            start: 0x1000, 
            end: 0x0FFF  // Invalid range (end < start)
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        
        let token = command.execute(&mut registers, &mut memory, &mut None).unwrap();
        
        // Should still get a View token with header and footer
        match token {
            OutputToken::View(lines) => {
                assert!(lines.len() >= 2); // At least header and footer
                assert!(lines[0].contains("Start of disassembly"));
                assert!(lines[1].contains("End of disassembly"));
            }
            _ => panic!("Expected OutputToken::View"),
        }
    }
}
