use std::path::PathBuf;
use std::env;

use anyhow::anyhow;
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use pest_derive::Parser;

use crate::{
    commands::*,
    until_condition::{Assignment, BooleanExpression, RegisterSource, Source},
    atari_binary::AtariBinary,
    apple_single::AppleSingle,
    SymbolTable,
    AppResult,
};

#[derive(Parser)]
#[grammar = "../rules.pest"]
pub struct PestParser;

pub struct ParserContext<'a> {
    symbols: Option<&'a SymbolTable>,
}

impl<'a> ParserContext<'a> {
    pub fn new(symbols: Option<&'a SymbolTable>) -> Self {
        Self { symbols }
    }

    fn prepare_hex_str(hex_str: &str) -> String {
        if hex_str.is_empty() {
            return String::new();
        }
        // If length is odd, pad with a leading zero
        if hex_str.len() % 2 == 1 {
            format!("0{}", hex_str)
        } else {
            hex_str.to_string()
        }
    }

    // Single function to parse hex string into bytes
    fn parse_hex_to_bytes(&self, hex_str: &str) -> AppResult<Vec<u8>> {
        if hex_str.is_empty() {
            return Err(anyhow!("Empty string is not a valid Hexadecimal."));
        }
        let hex_str = Self::prepare_hex_str(hex_str);
        hex::decode(&hex_str).map_err(|e| anyhow!("Failed to parse hex value '{}': {}", hex_str, e))
    }

    fn parse_hex(&self, hex_str: &str) -> AppResult<usize> {
        // Parse to bytes first
        let bytes = self.parse_hex_to_bytes(hex_str)?;
        // Convert bytes to usize (big endian - most significant byte first)
        let mut value = 0usize;
        for &byte in bytes.iter() {
            value = (value << 8) | (byte as usize);
        }
        Ok(value)
    }

    // Parse a comma-separated sequence of hex values into bytes
    // e.g., "F,FF,A" -> [0x0F, 0xFF, 0x0A]
    fn parse_hex_sequence(&self, sequence: &str) -> AppResult<Vec<u8>> {
        sequence
            .split(',')
            .map(|x| {
                let x = x.trim();
                // Parse each byte individually
                self.parse_hex_to_bytes(x).map(|mut v| v.remove(0))
            })
            .collect()
    }

    fn parse_memory(&self, pair: &Pair<Rule>) -> AppResult<usize> {
        match pair.as_rule() {
            Rule::memory_address => {
                let inner = pair.clone().into_inner().next().unwrap();
                self.parse_memory(&inner)
            }
            Rule::memory_location => {
                let inner = pair.clone().into_inner().next().unwrap();
                self.parse_memory(&inner)
            }
            Rule::hex_address => {
                let hex_str = &pair.as_str()[3..]; // Skip the "#0x" prefix
                self.parse_hex(hex_str)
            }
            Rule::symbol_reference => {
                let symbol_name = &pair.as_str()[1..]; // Skip the "$" prefix
                if let Some(symbols) = &self.symbols {
                    if let Some(addr) = symbols.get_address(symbol_name) {
                        return Ok(addr as usize);
                    }
                    return Err(anyhow!("Symbol '{}' not found", symbol_name));
                }
                Err(anyhow!("Symbol table not available for resolving '{}'", symbol_name))
            }
            _ => panic!("Unexpected rule in parse_memory: {:?}", pair.as_rule()),
        }
    }

    fn parse_source_memory(&self, node: &Pair<Rule>) -> AppResult<Source> {
        match node.as_rule() {
            Rule::memory_location => {
                let mut nodes = node.clone().into_inner();
                let addr_node = nodes.next().expect("memory_location should have a memory_address");
                let mut addr = self.parse_memory(&addr_node)?;
                
                // Check for optional offset
                if let Some(offset_node) = nodes.next() {
                    let mut offset_nodes = offset_node.into_inner();
                    let op_node = offset_nodes.next().unwrap();
                    let value_node = offset_nodes.next().unwrap();
                    
                    let offset = match value_node.as_rule() {
                        Rule::value8 | Rule::value16 => self.parse_source_value(&value_node)?,
                        _ => panic!("Unexpected offset type: {:?}", value_node.as_rule()),
                    };
                    
                    if let Source::Value(offset_val) = offset {
                        addr = match op_node.as_rule() {
                            Rule::plus_op => (addr.wrapping_add(offset_val)) & 0xFFFF,  // Mask to 16-bit
                            Rule::minus_op => (addr.wrapping_sub(offset_val)) & 0xFFFF, // Mask to 16-bit
                            _ => panic!("Unexpected operator type: {:?}", op_node.as_rule()),
                        };
                    }
                }
                
                Ok(Source::Memory(addr))
            },
            _ => panic!("Expected memory_location, got {:?}", node.as_rule()),
        }
    }

    fn parse_source_value(&self, node: &Pair<Rule>) -> AppResult<Source> {
        let value_str = node.as_str();
        let value = if value_str.starts_with("0x") {
            // Parse hex value
            self.parse_hex(&value_str[2..])?
        } else if value_str.starts_with("0b") {
            // Parse binary value
            usize::from_str_radix(&value_str[2..], 2)
                .map_err(|e| anyhow!("Failed to parse binary value '{}': {}", value_str, e))?
        } else {
            // Parse decimal value
            value_str.parse::<usize>()
                .map_err(|e| anyhow!("Failed to parse decimal value '{}': {}", value_str, e))?
        };
        
        // Validate the value size matches the rule type
        match node.as_rule() {
            Rule::value8 => {
                if value > 0xFF {
                    return Err(anyhow!("Value {} is too large for 8-bit value", value));
                }
            }
            Rule::value16 => {
                if value > 0xFFFF {
                    return Err(anyhow!("Value {} is too large for 16-bit value", value));
                }
            }
            _ => panic!("Unexpected rule in parse_source_value: {:?}", node.as_rule()),
        }
        
        Ok(Source::Value(value))
    }

    fn parse_string_literal(&self, str_content: &str) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut chars = str_content.chars();
        
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    match chars.next().expect("escape sequence should have a character after \\") {
                        'n' => bytes.push(b'\n'),
                        'r' => bytes.push(b'\r'),
                        't' => bytes.push(b'\t'),
                        '0' => bytes.push(0),
                        '"' => bytes.push(b'"'),
                        '\\' => bytes.push(b'\\'),
                        'x' => {
                            // Parse hex escape sequence \xAA
                            let hex1 = chars.next().expect("hex escape sequence should have two hex digits after \\x");
                            let hex2 = chars.next().expect("hex escape sequence should have two hex digits after \\x");
                            let hex_str = format!("{}{}", hex1, hex2);
                            match u8::from_str_radix(&hex_str, 16) {
                                Ok(byte_val) => bytes.push(byte_val),
                                Err(_) => panic!("invalid hex escape sequence '\\x{}'", hex_str),
                            }
                        }
                        '\n' => {
                            // Line continuation: backslash followed by newline
                            // Skip both the backslash and the newline - don't add anything to bytes
                        }
                        '\r' => {
                            // Handle Windows-style line endings (\r\n)
                            // Check if the next character is \n and consume it too
                            if chars.as_str().starts_with('\n') {
                                chars.next(); // consume the \n
                            }
                            // Skip the line continuation - don't add anything to bytes
                        }
                        c => panic!("unknown escape sequence '\\{}'", c),
                    }
                }
                c => bytes.push(c as u8),
            }
        }
        bytes
    }

    fn parse_memory_sequence(&self, node: Pair<Rule>) -> AppResult<BooleanExpression> {
        let mut seq_nodes = node.into_inner();
        let addr_node = seq_nodes.next().expect("memory_sequence should have a memory_location node");
        let addr = self.parse_source_memory(&addr_node)?;
        
        let sequence_node = seq_nodes.next().expect("memory_sequence should have a bytes_list or string_literal node");
        let bytes = match sequence_node.as_rule() {
            Rule::bytes_list => {
                let bytes_node = sequence_node.into_inner().next().expect("bytes_list should contain a bytes node");
                self.parse_hex_sequence(bytes_node.as_str())?
            }
            Rule::string_literal => {
                // Remove the quotes
                let str_content = &sequence_node.as_str()[1..sequence_node.as_str().len()-1];
                self.parse_string_literal(str_content)
            }
            _ => panic!("Expected bytes_list or string_literal in memory_sequence")
        };
        
        Ok(BooleanExpression::MemorySequence(addr, bytes))
    }

    pub fn parse_boolean_condition(&self, nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
        let mut nodes = nodes.peekable();
        
        // Get the first term
        let first_node = nodes.next().unwrap();
        let mut expr = self.parse_boolean_term(first_node.into_inner())?;

        // Process remaining nodes in pairs (OR_OP + term)
        while let Some(op) = nodes.next() {
            match op.as_rule() {
                Rule::OR_OP => {
                    let right_term = nodes.next().expect("Expected term after OR");
                    let right = self.parse_boolean_term(right_term.into_inner())?;
                    expr = BooleanExpression::Or(Box::new(expr), Box::new(right));
                },
                _ => panic!("Unexpected rule in boolean condition: {:?}", op.as_rule()),
            }
        }

        Ok(expr)
    }

    fn parse_boolean_term(&self, nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
        let mut nodes = nodes.peekable();
        
        // Get the first factor
        let first_node = nodes.next().unwrap();
        let mut expr = self.parse_boolean_factor(first_node)?;

        // Process remaining nodes in pairs (AND_OP + factor)
        while let Some(op) = nodes.next() {
            match op.as_rule() {
                Rule::AND_OP => {
                    let right_factor = nodes.next().expect("Expected factor after AND");
                    let right = self.parse_boolean_factor(right_factor)?;
                    expr = BooleanExpression::And(Box::new(expr), Box::new(right));
                },
                _ => panic!("Unexpected rule in boolean term: {:?}", op.as_rule()),
            }
        }

        Ok(expr)
    }

    fn parse_boolean_factor(&self, node: Pair<Rule>) -> AppResult<BooleanExpression> {
        let mut inner = node.into_inner();
        let first = inner.next().unwrap();
        
        match first.as_rule() {
            Rule::comparison => self.parse_comparison(first.into_inner()),
            Rule::boolean_condition => self.parse_boolean_condition(first.into_inner()),
            Rule::memory_sequence => self.parse_memory_sequence(first),
            Rule::pointer_assertion => self.parse_pointer_assertion(first),
            Rule::boolean => Ok(BooleanExpression::Value(first.as_str() == "true")),
            Rule::NOT_OP => {
                let factor = inner.next().expect("NOT operator should be followed by a factor");
                Ok(BooleanExpression::Not(Box::new(self.parse_boolean_factor(factor)?)))
            },
            _ => panic!("unknown node type '{:?}'. Is the Pest grammar up to date?", first.as_rule()),
        }
    }

    fn parse_pointer_assertion(&self, node: Pair<Rule>) -> AppResult<BooleanExpression> {
        let mut nodes = node.into_inner();
        
        // Get the pointer location (memory address where the pointer is stored)
        let pointer_loc = nodes.next().expect("pointer_assertion should have a memory address");
        let pointer_addr = self.parse_memory(&pointer_loc)?;

        // Skip the "points" and "to" tokens by getting the pointer_target
        let target_node = nodes.next().expect("pointer_assertion should have a pointer_target");
        let mut target_nodes = target_node.into_inner();
        
        // Get the base target address
        let target_addr_node = target_nodes.next().expect("pointer_target should have a memory address");
        let mut target_addr = self.parse_memory(&target_addr_node)?;

        // Check for optional offset
        if let Some(offset_node) = target_nodes.next() {
            let mut offset_nodes = offset_node.into_inner();
            let op_node = offset_nodes.next().unwrap();
            let value_node = offset_nodes.next().unwrap();
            
            let offset = match value_node.as_rule() {
                Rule::value8 | Rule::value16 => self.parse_source_value(&value_node)?,
                _ => panic!("Unexpected offset type: {:?}", value_node.as_rule()),
            };
            
            if let Source::Value(offset_val) = offset {
                target_addr = match op_node.as_rule() {
                    Rule::plus_op => target_addr.wrapping_add(offset_val),
                    Rule::minus_op => target_addr.wrapping_sub(offset_val),
                    _ => panic!("Unexpected operator type: {:?}", op_node.as_rule()),
                };
            }
        }

        // For 6502, pointers are stored in little-endian format
        // So we need to check that pointer_addr contains the low byte
        // and pointer_addr + 1 contains the high byte
        let low_byte = target_addr & 0xFF;
        let high_byte = (target_addr >> 8) & 0xFF;

        // Create a boolean expression that checks both bytes
        let low_check = BooleanExpression::Equal(
            Source::Memory(pointer_addr),
            Source::Value(low_byte)
        );
        let high_check = BooleanExpression::Equal(
            Source::Memory(pointer_addr.wrapping_add(1)),
            Source::Value(high_byte)
        );

        // Combine the checks with AND
        Ok(BooleanExpression::And(
            Box::new(low_check),
            Box::new(high_check)
        ))
    }

    fn parse_comparison(&self, mut nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
        let lh_node = nodes.next().unwrap();
        let lh = match lh_node.as_rule() {
            Rule::register8 | Rule::register16 | Rule::register_cycle => self.parse_source_register(&lh_node),
            Rule::memory_location => self.parse_source_memory(&lh_node)?,
            Rule::value8 | Rule::value16 => self.parse_source_value(&lh_node)?,
            v => panic!("unexpected node '{:?}' in comparison", v),
        };

        let op = nodes.next().unwrap();
        let rh_node = nodes.next().unwrap();
        let rh = match rh_node.as_rule() {
            Rule::register8 | Rule::register16 | Rule::register_cycle => self.parse_source_register(&rh_node),
            Rule::memory_location => self.parse_source_memory(&rh_node)?,
            Rule::value8 | Rule::value16 => self.parse_source_value(&rh_node)?,
            Rule::symbol_reference => {
                let symbol_name = &rh_node.as_str()[1..]; // Skip the "$" prefix
                if let Some(symbols) = &self.symbols {
                    if let Some(addr) = symbols.get_address(symbol_name) {
                        Source::Value(addr as usize)
                    } else {
                        return Err(anyhow!("Symbol '{}' not found", symbol_name));
                    }
                } else {
                    return Err(anyhow!("Symbol table not available for resolving '{}'", symbol_name));
                }
            },
            Rule::symbol_byte_reference => {
                let inner = rh_node.clone().into_inner().next().unwrap();
                match inner.as_rule() {
                    Rule::symbol_low_byte => {
                        let symbol_name = &inner.as_str()[2..]; // Skip the "<$" prefix
                        if let Some(symbols) = &self.symbols {
                            if let Some(addr) = symbols.get_address(symbol_name) {
                                Source::Value((addr as usize) & 0xFF) // Low byte
                            } else {
                                return Err(anyhow!("Symbol '{}' not found", symbol_name));
                            }
                        } else {
                            return Err(anyhow!("Symbol table not available for resolving '{}'", symbol_name));
                        }
                    },
                    Rule::symbol_high_byte => {
                        let symbol_name = &inner.as_str()[2..]; // Skip the ">$" prefix
                        if let Some(symbols) = &self.symbols {
                            if let Some(addr) = symbols.get_address(symbol_name) {
                                Source::Value((addr as usize) >> 8) // High byte
                            } else {
                                return Err(anyhow!("Symbol '{}' not found", symbol_name));
                            }
                        } else {
                            return Err(anyhow!("Symbol table not available for resolving '{}'", symbol_name));
                        }
                    },
                    v => panic!("unexpected symbol byte reference type '{:?}'", v),
                }
            },
            v => panic!("unexpected node '{:?}' in comparison", v),
        };

        Ok(match op.as_str() {
            "=" => BooleanExpression::Equal(lh, rh),
            ">=" => BooleanExpression::GreaterOrEqual(lh, rh),
            ">" => BooleanExpression::StrictlyGreater(lh, rh),
            "<=" => BooleanExpression::LesserOrEqual(lh, rh),
            "<" => BooleanExpression::StrictlyLesser(lh, rh),
            "!=" => BooleanExpression::Different(lh, rh),
            v => panic!("unknown operator {:?}", v),
        })
    }

    fn parse_source_register(&self, node: &Pair<Rule>) -> Source {
        match node.as_str() {
            "A" => Source::Register(RegisterSource::Accumulator),
            "X" => Source::Register(RegisterSource::RegisterX),
            "Y" => Source::Register(RegisterSource::RegisterY),
            "S" => Source::Register(RegisterSource::Status),
            "SP" => Source::Register(RegisterSource::StackPointer),
            "CP" => Source::Register(RegisterSource::CommandPointer),
            "cycle_count" => Source::Register(RegisterSource::CycleCount),
            v => panic!("unknown register type '{:?}'.", v),
        }
    }
}

pub struct MemoryCommandParser<'a> {
    context: &'a ParserContext<'a>,
}

impl<'a> MemoryCommandParser<'a> {
    pub fn new(context: &'a ParserContext<'a>) -> Self {
        Self { context }
    }

    pub fn from_pairs(pairs: Pairs<'_, Rule>, context: &'a ParserContext<'a>) -> AppResult<MemoryCommand> {
        let parser = Self::new(context);
        parser.parse_pairs(pairs)
    }

    fn parse_pairs(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let pair = pairs.next().unwrap();

        let command = match pair.as_rule() {
            Rule::memory_flush => MemoryCommand::Flush,
            Rule::memory_write => self.handle_memory_write(pair.into_inner())?,
            Rule::memory_load => self.handle_memory_load(pair.into_inner())?,
            Rule::memory_fill => self.handle_memory_fill(pair.into_inner())?,
            Rule::symbol_load => self.handle_symbol_load(pair.into_inner())?,
            Rule::symbol_add => self.handle_symbol_add(pair.into_inner())?,
            Rule::symbol_remove => self.handle_symbol_remove(pair.into_inner())?,
            Rule::memory_show => self.handle_memory_show(pair.into_inner())?,
            _ => return Err(anyhow!("Unknown memory command")),
        };

        Ok(command)
    }

    fn handle_memory_write(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let addr_pair = pairs
            .next()
            .expect("there shall be a memory address argument to memory write");
        
        let address = match addr_pair.as_rule() {
            Rule::memory_address => {
                self.context.parse_memory(&addr_pair)?
            }
            _ => panic!("Expected memory_address, got {:?}", addr_pair.as_rule()),
        };
        
        let bytes_node = pairs
            .next()
            .expect("There shall be some bytes to write to memory");

        let bytes = match bytes_node.as_rule() {
            Rule::bytes => self.context.parse_hex_sequence(bytes_node.as_str())?,
            Rule::string_literal => {
                // Remove the quotes
                let str_content = &bytes_node.as_str()[1..bytes_node.as_str().len()-1];
                self.context.parse_string_literal(str_content)
            }
            Rule::memory_location => {
                let address = self.context.parse_memory(&bytes_node)?;
                vec![(address & 0xFF) as u8, (address >> 8) as u8]
            }
            _ => panic!("Expected bytes, string_literal, or memory_location in memory write")
        };

        Ok(MemoryCommand::Write { address, bytes })
    }

    fn handle_memory_load(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let first_arg = pairs
            .next()
            .expect("there shall be a memory address or target argument to memory load");

        match first_arg.as_rule() {
            Rule::target_name => self.handle_target_load(first_arg, pairs.next()),
            Rule::memory_address => self.handle_address_load(first_arg, pairs.next()),
            _ => panic!("Unexpected first argument to memory load"),
        }
    }

    fn expand_env_vars(path: &str) -> String {
        let mut result = path.to_string();
        
        // Handle ${VAR} style
        while let Some(start) = result.find("${") {
            if let Some(end) = result[start..].find('}') {
                let var_name = &result[start + 2..start + end];
                let var_value = env::var(var_name).unwrap_or_else(|_| String::new());
                result.replace_range(start..start + end + 1, &var_value);
            } else {
                break;
            }
        }
        
        // Handle $VAR style
        while let Some(start) = result.find('$') {
            if start + 1 >= result.len() {
                break;
            }
            
            // Find the end of the variable name
            let mut end = start + 1;
            while end < result.len() && (result[end..end + 1].chars().next().unwrap().is_alphanumeric() || result[end..end + 1].chars().next().unwrap() == '_') {
                end += 1;
            }
            
            if end > start + 1 {
                let var_name = &result[start + 1..end];
                let var_value = env::var(var_name).unwrap_or_else(|_| String::new());
                result.replace_range(start..end, &var_value);
            } else {
                break;
            }
        }
        
        result
    }

    fn handle_address_load(&self, address_pair: Pair<'_, Rule>, filename_pair: Option<Pair<'_, Rule>>) -> AppResult<MemoryCommand> {
        println!("DEBUG handle_address_load:");
        let address = self.context.parse_memory(&address_pair)?;
        let filename = filename_pair
            .expect("there shall be a filename argument to memory load")
            .as_str();
        let stripped = &filename[1..filename.len() - 1];
        let expanded = Self::expand_env_vars(stripped);
        let filepath = PathBuf::from(expanded);

        Ok(MemoryCommand::Load { address, filepath })
    }

    fn handle_target_load(&self, target_pair: Pair<'_, Rule>, filename_pair: Option<Pair<'_, Rule>>) -> AppResult<MemoryCommand> {
        let target = target_pair.as_str();
        let filename_pair = filename_pair.expect("there shall be a filename argument to memory load");
        let filename = filename_pair.as_str();
        let stripped = &filename[1..filename.len() - 1];
        let expanded = Self::expand_env_vars(stripped);
        let filepath = PathBuf::from(expanded);

        let command = match target {
            "atari" => {
                let binary = AtariBinary::from_file(&filepath)?;
                let segments = binary.into_memory_segments();
                MemoryCommand::LoadSegments { segments }
            }
            "apple" => {
                let binary = AppleSingle::from_file(&filepath)?;
                let segments = binary.into_memory_segments();
                MemoryCommand::LoadSegments { segments }
            }
            // This case is unreachable because the grammar only allows "atari" or "apple"
            _ => unreachable!("Grammar ensures only 'atari' or 'apple' can be targets"),
        };

        Ok(command)
    }

    fn handle_symbol_load(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let filename = pairs
            .next()
            .expect("there shall be a filename argument to symbols load")
            .as_str();
        let stripped = &filename[1..filename.len() - 1];
        let expanded = Self::expand_env_vars(stripped);
        let filepath = PathBuf::from(expanded);
        
        let mut symbols = SymbolTable::new();
        symbols.load_vice_labels(&filepath)?;
        Ok(MemoryCommand::LoadSymbols { symbols })
    }

    fn handle_symbol_add(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let name = pairs.next().unwrap().as_str().to_string();
        let value_node = pairs.next().unwrap();
        
        // First get the inner value from symbol_add_value
        let value_inner = value_node.into_inner().next().unwrap();

        let value = match value_inner.as_rule() {
            Rule::value8 | Rule::value16 => {
                let value_str = &value_inner.as_str()[2..]; // Skip "0x"
                self.context.parse_hex(value_str)?
            }
            Rule::symbol_reference => {
                self.context.parse_memory(&value_inner)?
            }
            _ => panic!("Unexpected value type in symbol_add: {:?}", value_inner.as_rule()),
        };

        Ok(MemoryCommand::AddSymbol { 
            name, 
            value: value as u16 
        })
    }

    fn handle_symbol_remove(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let name = pairs.next().unwrap().as_str().to_string();
        Ok(MemoryCommand::RemoveSymbol { name })
    }

    fn handle_memory_fill(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let start_node = pairs.next().expect("memory_fill should have a start address");
        let start = match start_node.as_rule() {
            Rule::memory_location => {
                let source = self.context.parse_source_memory(&start_node)?;
                if let Source::Memory(addr) = source {
                    addr
                } else {
                    panic!("Expected memory location for fill start address");
                }
            }
            _ => panic!("Expected memory_location for fill start address"),
        };

        let end_node = pairs.next().expect("memory_fill should have an end address");
        let end = match end_node.as_rule() {
            Rule::memory_location => {
                let source = self.context.parse_source_memory(&end_node)?;
                if let Source::Memory(addr) = source {
                    addr
                } else {
                    panic!("Expected memory location for fill end address");
                }
            }
            _ => panic!("Expected memory_location for fill end address"),
        };

        // Optional fill value, defaults to 0
        let value = if let Some(value_node) = pairs.next() {
            match self.context.parse_source_value(&value_node)? {
                Source::Value(v) => v as u8,
                _ => panic!("Expected value for fill"),
            }
        } else {
            0
        };

        Ok(MemoryCommand::Fill { start, end, value })
    }

    fn handle_memory_show(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let addr_pair = pairs.next().unwrap();
        let address = match addr_pair.as_rule() {
            Rule::memory_location => {
                let source = self.context.parse_source_memory(&addr_pair)?;
                if let Source::Memory(addr) = source {
                    addr
                } else {
                    return Err(anyhow!("Expected memory location for show address"));
                }
            }
            _ => return Err(anyhow!("Expected memory_location, got {:?}", addr_pair.as_rule())),
        };

        let length_pair = pairs.next().unwrap();
        let length = match length_pair.as_rule() {
            Rule::value8 | Rule::value16 => {
                match self.context.parse_source_value(&length_pair)? {
                    Source::Value(v) => v,
                    _ => return Err(anyhow!("Expected value for length")),
                }
            }
            _ => return Err(anyhow!("Expected value8 or value16 for length")),
        };

        // Parse optional width parameter
        let mut width = None;
        let mut description = None;
        
        if let Some(next_pair) = pairs.next() {
            match next_pair.as_rule() {
                Rule::value8 => {
                    // This is the width parameter
                    match self.context.parse_source_value(&next_pair)? {
                        Source::Value(w) => {
                            width = Some(w);
                            // Parse optional description after width
                            if let Some(desc_pair) = pairs.next() {
                                description = Some(desc_pair.as_str().to_string());
                            }
                        }
                        _ => return Err(anyhow!("Expected value for width")),
                    }
                }
                Rule::description => {
                    // This is the description (no width provided)
                    description = Some(next_pair.as_str().to_string());
                }
                _ => return Err(anyhow!("Unexpected rule in memory show: {:?}", next_pair.as_rule())),
            }
        }

        Ok(MemoryCommand::Show { address, length, width, description })
    }
}

#[cfg(test)]
mod test_utils {
    use super::*;

    pub fn setup_test_symbols() -> SymbolTable {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x34, "byte_var".to_string());    // 8-bit value
        symbols.add_symbol(0x1234, "word_var".to_string());  // 16-bit value
        symbols.add_symbol(0x2000, "counter".to_string());
        symbols
    }
}

#[cfg(test)]
mod memory_command_parser_tests {
    use super::*;

    fn create_test_context<'a>() -> ParserContext<'a> {
        ParserContext::new(None)
    }

    #[test]
    fn test_memory_flush() {
        let input = "memory flush";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command, MemoryCommand::Flush));
    }

    #[test]
    fn test_memory_write() {
        let input = "memory write #0x1234 0x(01,02,03)";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } if address == 0x1234 && bytes == vec![0x01, 0x02, 0x03])
        );
    }

    #[test]
    fn test_memory_write_with_symbols() {
        let symbols = test_utils::setup_test_symbols();
        let context = ParserContext::new(Some(&symbols));
        let input = "memory write $word_var 0x(a9,c0)";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == vec![0xa9, 0xc0])
        );
    }

    #[test]
    fn test_memory_write_with_memory_location() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "somewhere".to_string());
        symbols.add_symbol(0x1234, "address".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "memory write $somewhere $address";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1000 && bytes == vec![0x34, 0x12])
        );
    }

    #[test]
    fn test_memory_write_with_string() {
        let input = "memory write #0x1234 \"hello\"";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == b"hello".to_vec())
        );
    }

    #[test]
    fn test_memory_write_with_string_escapes() {
        let context = create_test_context();
        
        // Test null terminator
        let input = "memory write #0x1234 \"hello\\0\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == b"hello\0".to_vec())
        );

        // Test other escape sequences
        let input = "memory write #0x1234 \"hello\\n\\r\\t\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == b"hello\n\r\t".to_vec())
        );

        // Test escaped quotes and backslashes
        let input = "memory write #0x1234 \"\\\"hello\\\\\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == b"\"hello\\".to_vec())
        );
    }

    #[test]
    fn test_memory_write_with_hex_escape_sequences() {
        let context = create_test_context();
        
        // Test basic hex escape sequences
        let input = "memory write #0x1234 \"\\xFF\\x00\\x42\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == vec![0xFF, 0x00, 0x42])
        );

        // Test mixed text and hex escape sequences
        let input = "memory write #0x1234 \"hello\\x00world\\x0A\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        let expected = b"hello\0world\x0A".to_vec();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == expected)
        );

        // Test hex escape sequences with both cases
        let input = "memory write #0x1234 \"\\xAB\\xcd\\x12\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == vec![0xAB, 0xCD, 0x12])
        );

        // Test mixing hex escapes with standard escapes
        let input = "memory write #0x1234 \"data:\\x0A\\t\\xFF\\n\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        let expected = b"data:\x0A\t\xFF\n".to_vec();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == expected)
        );
    }

    #[test]
    fn test_memory_write_with_line_continuation() {
        let context = create_test_context();
        
        // Test basic line continuation
        let input = "memory write #0x1234 \"hello\\\nworld\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1234 && bytes == b"helloworld".to_vec())
        );

        // Test multi-line with line continuations (simulating screen layout)
        let input = "memory write #0x1000 \"+-------+\\\n| Hello |\\\n+-------+\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        let expected = b"+-------+| Hello |+-------+".to_vec();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x1000 && bytes == expected)
        );

        // Test line continuation with spaces preserved
        let input = "memory write #0x2000 \"start \\\n  middle \\\n    end\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        let expected = b"start   middle     end".to_vec();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x2000 && bytes == expected)
        );

        // Test mixing line continuation with other escape sequences
        let input = "memory write #0x3000 \"line1\\\n\\ttab\\\n\\nend\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        let expected = b"line1\ttab\nend".to_vec();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x3000 && bytes == expected)
        );
    }

    #[test]
    fn test_memory_load() {
        let input = "memory load #0x1000 \"script.txt\"";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Load { address, filepath } if address == 0x1000 && filepath == PathBuf::from("script.txt"))
        );
    }

    #[test]
    fn test_memory_load_with_symbols() {
        let symbols = test_utils::setup_test_symbols();
        let context = ParserContext::new(Some(&symbols));
        let input = "memory load $word_var \"script.txt\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Load { address, filepath } if address == 0x1234 && filepath == PathBuf::from("script.txt"))
        );
    }

    #[test]
    fn test_memory_load_target_parsing() {

        // Test Atari target loading
        let pairs = PestParser::parse(Rule::memory_instruction, "memory load atari \"test.com\"")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let pair = pairs.into_iter().next().unwrap();

        // Verify it's the correct rule type
        assert!(matches!(pair.as_rule(), Rule::memory_load));

        // Verify the inner parts (target and filename)
        let mut inner = pair.into_inner();
        let target = inner.next().unwrap();
        assert_eq!(target.as_str(), "atari");
        assert!(matches!(target.as_rule(), Rule::target_name));
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"test.com\"");

        // Test Apple target loading
        let pairs = PestParser::parse(Rule::memory_instruction, "memory load apple \"test.as\"")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let pair = pairs.into_iter().next().unwrap();

        // Verify it's the correct rule type
        assert!(matches!(pair.as_rule(), Rule::memory_load));

        // Verify the inner parts (target and filename)
        let mut inner = pair.into_inner();
        let target = inner.next().unwrap();
        assert_eq!(target.as_str(), "apple");
        assert!(matches!(target.as_rule(), Rule::target_name));
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"test.as\"");

        // Also verify that invalid targets are rejected by the grammar
        assert!(PestParser::parse(Rule::memory_instruction, "memory load invalid_target \"test.txt\"").is_err());
    }

    #[test]
    fn test_target_name_validation() {
        // Test that only valid targets are accepted by the grammar
        assert!(PestParser::parse(Rule::target_name, "atari").is_ok());
        assert!(PestParser::parse(Rule::target_name, "apple").is_ok());
        assert!(PestParser::parse(Rule::target_name, "invalid_target").is_err());
    }

    #[test]
    fn test_memory_load_with_hyphenated_path() {
        
        // Test parsing of memory load with target and hyphenated path
        let input = "memory load atari \"path/with-hyphen/test.bin\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        
        // Just verify the parsing succeeds and produces the expected command type and path
        let pair = pairs.into_iter().next().unwrap();
        assert!(matches!(pair.as_rule(), Rule::memory_load));
        let mut inner = pair.into_inner();
        let target = inner.next().unwrap();
        assert_eq!(target.as_str(), "atari");
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"path/with-hyphen/test.bin\"");

        // Test parsing of memory load with address and hyphenated path
        let input = "memory load #0x1234 \"another-path/test.bin\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        
        // Verify address load parsing
        let pair = pairs.into_iter().next().unwrap();
        assert!(matches!(pair.as_rule(), Rule::memory_load));
        let mut inner = pair.into_inner();
        let addr = inner.next().unwrap();
        assert!(matches!(addr.as_rule(), Rule::memory_address));
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"another-path/test.bin\"");

        // Test parsing with environment variables in path
        let input = "memory load #0x1234 \"${BUILD_DIR}/test-file.bin\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        
        // Verify environment variable path parsing
        let pair = pairs.into_iter().next().unwrap();
        assert!(matches!(pair.as_rule(), Rule::memory_load));
        let mut inner = pair.into_inner();
        let addr = inner.next().unwrap();
        assert!(matches!(addr.as_rule(), Rule::memory_address));
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"${BUILD_DIR}/test-file.bin\"");
    }

    #[test]
    fn test_memory_fill() {
        let context = create_test_context();
        
        // Test basic fill with value
        let input = "memory fill #0x1000~#0x1FFF 0x42";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Fill { start, end, value } 
                if start == 0x1000 && end == 0x1FFF && value == 0x42)
        );

        // Test fill without value (should default to 0)
        let input = "memory fill #0x1000~#0x1FFF";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Fill { start, end, value } 
                if start == 0x1000 && end == 0x1FFF && value == 0)
        );

        // Test fill with symbols and offsets
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "array".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "memory fill $array + 2 ~ $array + 5 0xFF";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Fill { start, end, value } 
                if start == 0x1002 && end == 0x1005 && value == 0xFF)
        );

        // Test fill with decimal value
        let input = "memory fill #0x1000~#0x1FFF 42";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, MemoryCommand::Fill { start, end, value } 
                if start == 0x1000 && end == 0x1FFF && value == 42)
        );
    }

    #[test]
    fn test_memory_show() {
        let context = create_test_context();
        
        // Test with hex address and 8-bit length
        let input = "memory show #0x1234 0x10";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Show { address, length, width, description }
                if address == 0x1234 && length == 0x10 && width.is_none() && description.is_none()
            )
        );

        // Test with symbol and 16-bit length
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "data".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "memory show $data 0x100";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Show { address, length, width, description }
                if address == 0x1000 && length == 0x100 && width.is_none() && description.is_none()
            )
        );

        // Test with decimal length
        let input = "memory show #0x1234 16";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Show { address, length, width, description }
                if address == 0x1234 && length == 16 && width.is_none() && description.is_none()
            )
        );

        // Test with description
        let input = "memory show #0x1234 16 $$Stack contents$$";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Show { address, length, width, description }
                if address == 0x1234 && length == 16 && width.is_none() && description.as_deref() == Some("Stack contents")
            )
        );

        // Test with width
        let input = "memory show #0x1234 16 8";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Show { address, length, width, description }
                if address == 0x1234 && length == 16 && width == Some(8) && description.is_none()
            )
        );

        // Test with width and description
        let input = "memory show #0x1234 16 4 $$Data with custom width$$";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Show { address, length, width, description }
                if address == 0x1234 && length == 16 && width == Some(4) && description.as_deref() == Some("Data with custom width")
            )
        );
    }
}

#[cfg(test)]
mod symbol_command_parser_tests {
    use super::*;

    #[test]
    fn test_symbol_load_command_parsing() {
        let input = "symbols load \"test.sym\"";
        let pairs = PestParser::parse(Rule::symbols_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let pair = pairs.into_iter().next().unwrap();

        // Verify it's the correct rule type
        assert!(matches!(pair.as_rule(), Rule::symbol_load));

        // Verify the filename
        let mut inner = pair.into_inner();
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"test.sym\"");
    }

    #[test]
    fn test_symbol_add_with_value8() {
        let input = "symbols add foo=0x12";
        let context = ParserContext::new(None);
        let pairs = PestParser::parse(Rule::symbols_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(
            command,
            MemoryCommand::AddSymbol { name, value }
            if name == "foo" && value == 0x12
        ));
    }

    #[test]
    fn test_symbol_add_with_value16() {
        let input = "symbols add bar=0x1234";
        let context = ParserContext::new(None);
        let pairs = PestParser::parse(Rule::symbols_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(
            command,
            MemoryCommand::AddSymbol { name, value }
            if name == "bar" && value == 0x1234
        ));
    }

    #[test]
    fn test_symbol_add_with_reference() {
        // First create a context with the referenced symbol
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x12, "foo".to_string());
        let context = ParserContext::new(Some(&symbols));

        let input = "symbols add baz=$foo";
        let pairs = PestParser::parse(Rule::symbols_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(
            command,
            MemoryCommand::AddSymbol { name, value }
            if name == "baz" && value == 0x12
        ));
    }

    #[test]
    fn test_symbol_remove_parser() {
        let cli_command = CliCommandParser::from("symbols remove foo").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::RemoveSymbol { name })
            if name == "foo"
        ));
    }

    #[test]
    fn test_symbol_remove_with_existing_symbol() {
        // First create a context with a symbol
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x1234, "foo".to_string());
        let context = ParserContext::new(Some(&symbols));

        let input = "symbols remove foo";
        let pairs = PestParser::parse(Rule::symbols_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(
            command,
            MemoryCommand::RemoveSymbol { name }
            if name == "foo"
        ));
    }
}

pub struct RegisterCommandParser<'a> {
    context: &'a ParserContext<'a>,
}

impl<'a> RegisterCommandParser<'a> {
    pub fn new(context: &'a ParserContext<'a>) -> Self {
        Self { context }
    }

    pub fn from_pairs(pairs: Pairs<'_, Rule>, context: &'a ParserContext<'a>) -> AppResult<RegisterCommand> {
        let parser = Self::new(context);
        parser.parse_pairs(pairs)
    }

    fn parse_pairs(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<RegisterCommand> {
        let pair = pairs.next().unwrap();

        let command = match pair.as_rule() {
            Rule::registers_flush => RegisterCommand::Flush,
            Rule::registers_set => self.parse_register_set(pair.into_inner())?,
            Rule::registers_show => self.parse_register_show(pair.into_inner())?,
            _ => {
                panic!("Unexpected rule '{}', register rule was expected.", pair);
            }
        };

        Ok(command)
    }

    fn parse_register_set(&self, mut pairs: Pairs<Rule>) -> AppResult<RegisterCommand> {
        let assignment = pairs.next().unwrap();
        let mut assignment = assignment.into_inner();
        let destination_node = assignment
            .next()
            .ok_or_else(|| anyhow!("expected a destination for register assignment"))?;
        
        let (destination, is_16bit) = match destination_node.as_rule() {
            Rule::register8 => (match destination_node.as_str() {
                "A" => RegisterSource::Accumulator,
                "X" => RegisterSource::RegisterX,
                "Y" => RegisterSource::RegisterY,
                "S" => RegisterSource::Status,
                "SP" => RegisterSource::StackPointer,
                v => panic!("unknown destination 8 bits register type '{:?}'.", v),
            }, false),
            Rule::register16 => (match destination_node.as_str() {
                "CP" => RegisterSource::CommandPointer,
                v => panic!("unknown destination 16 bits register type '{:?}'.", v),
            }, true),
            Rule::register_cycle => (RegisterSource::CycleCount, true),
            v => panic!("unexpected node '{:?}' here.", v),
        };

        let source_node = assignment.next().unwrap();
        let source = match source_node.as_rule() {
            Rule::register8 | Rule::register16 | Rule::register_cycle => self.parse_source_register(&source_node),
            Rule::value8 | Rule::value16 => self.context.parse_source_value(&source_node)?,
            Rule::memory_address => {
                // For symbols, get the value and validate it against register size
                let value = self.context.parse_memory(&source_node)?;
                if !is_16bit && value > 0xFF {
                    return Err(anyhow!("Value 0x{:X} is too large for 8-bit register", value));
                }
                Source::Value(value)
            },
            v => panic!("unexpected node '{:?}' here.", v),
        };

        Ok(RegisterCommand::Set {
            assignment: Assignment {
                destination,
                source,
            },
        })
    }

    fn parse_register_show(&self, mut pairs: Pairs<Rule>) -> AppResult<RegisterCommand> {
        let register = pairs.next()
            .map(|pair| self.parse_register_source(pair.as_str()));

        Ok(RegisterCommand::Show { register })
    }

    fn parse_source_register(&self, node: &Pair<Rule>) -> Source {
        Source::Register(self.parse_register_source(node.as_str()))
    }

    fn parse_register_source(&self, register_str: &str) -> RegisterSource {
        match register_str {
            "A" => RegisterSource::Accumulator,
            "X" => RegisterSource::RegisterX,
            "Y" => RegisterSource::RegisterY,
            "S" => RegisterSource::Status,
            "SP" => RegisterSource::StackPointer,
            "CP" => RegisterSource::CommandPointer,
            "cycle_count" => RegisterSource::CycleCount,
            v => panic!("unknown register type '{:?}'.", v),
        }
    }
}

#[cfg(test)]
mod register_parser_tests {
    use super::*;

    fn create_test_context<'a>() -> ParserContext<'a> {
        ParserContext::new(None)
    }

    #[test]
    fn test_registers_flush() {
        let input = "registers flush";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command, RegisterCommand::Flush));
    }

    #[test]
    fn test_registers_show() {
        let input = "registers show";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command, RegisterCommand::Show { register: None }));
    }

    #[test]
    fn test_registers_show_cycle_count() {
        let input = "registers show cycle_count";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command, RegisterCommand::Show { register: Some(RegisterSource::CycleCount) }));
    }

    #[test]
    fn test_registers_show_accumulator() {
        let input = "registers show A";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command, RegisterCommand::Show { register: Some(RegisterSource::Accumulator) }));
    }

    #[test]
    fn test_registers_set_value8() {
        let input = "registers set A=0xc0";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::Accumulator)
                && matches!(assignment.source, Source::Value(d) if d == 0xc0)
            )
        );
    }

    #[test]
    fn test_registers_set_value8_with_symbol_value() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x12, "test_var".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "registers set A=$test_var";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::Accumulator)
                && matches!(assignment.source, Source::Value(d) if d == 0x12)
            )
        );
    }

    #[test]
    fn test_registers_set_value8_with_symbol_value_too_large() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x1234, "test_var".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "registers set A=$test_var";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let result = RegisterCommandParser::from_pairs(pairs, &context);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large for 8-bit register"));
    }

    #[test]
    fn test_registers_set_value16() {
        let input = "registers set CP=0xc0ff";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::CommandPointer)
                && matches!(assignment.source, Source::Value(d) if d == 0xc0ff)
            )
        );
    }

    #[test]
    fn test_registers_set_value16_with_symbol_value() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x1234, "test_var".to_string());  // Valid 16-bit value
        let context = ParserContext::new(Some(&symbols));
        
        let input = "registers set CP=$test_var";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::CommandPointer)
                && matches!(assignment.source, Source::Value(0x1234))
            )
        );
    }

    #[test]
    fn test_registers_set_cycle_count() {
        let input = "registers set cycle_count=0x1234";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::CycleCount)
                && matches!(assignment.source, Source::Value(0x1234))
            )
        );
    }

    #[test]
    fn test_registers_set_cycle_count_zero() {
        let input = "registers set cycle_count=0x0000";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::CycleCount)
                && matches!(assignment.source, Source::Value(0))
            )
        );
    }

    #[test]
    fn test_registers_set_value8_decimal() {
        let input = "registers set A=192"; // 192 = 0xC0
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::Accumulator)
                && matches!(assignment.source, Source::Value(d) if d == 0xc0)
            )
        );
    }

    #[test]
    fn test_registers_set_cycle_count_decimal() {
        let input = "registers set cycle_count=256"; // Test the boundary value
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::CycleCount)
                && matches!(assignment.source, Source::Value(0x100))
            )
        );
    }

    #[test]
    fn test_run_with_cycle_count_decimal() {
        let input = "run while cycle_count < 256";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command.stop_condition, BooleanExpression::Value(false)));
        assert!(matches!(command.continue_condition,
            BooleanExpression::StrictlyLesser(
                Source::Register(RegisterSource::CycleCount),
                Source::Value(256)
            )
        ));
    }

    #[test]
    fn test_value8_decimal_too_large() {
        let input = "registers set A=256"; // Too large for 8-bit
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let result = RegisterCommandParser::from_pairs(pairs, &context);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("too large for 8-bit value"));
    }

    #[test]
    fn test_registers_set_value8_single_digit_hex() {
        let input = "registers set A=0xF"; // Test single digit hex
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::Accumulator)
                && matches!(assignment.source, Source::Value(d) if d == 0x0F)
            )
        );
    }

    #[test]
    fn test_memory_write_with_single_digit_hex() {
        let input = "memory write #0x1234 0x(F,A,C)";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                MemoryCommand::Write { address, bytes }
                if address == 0x1234 && bytes == vec![0x0F, 0x0A, 0x0C]
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence_single_digit_hex() {
        let context = create_test_context();
        let input = "assert #0x8000 ~ 0x(1,A,F) $$check memory sequence with single digit hex$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check memory sequence with single digit hex");
        
        // Extract the actual values from the condition for better error messages
        match &command.condition {
            BooleanExpression::MemorySequence(source, bytes) => {
                match source {
                    Source::Memory(addr) => {
                        assert_eq!(*addr, 0x8000, "Expected address 0x8000, got 0x{:04X}", addr);
                    },
                    _ => panic!("Expected Source::Memory, got {:?}", source),
                }
                assert_eq!(bytes, &vec![0x01, 0x0A, 0x0F], 
                    "Expected bytes [0x01, 0x0A, 0x0F], got {:?}", 
                    bytes.iter().map(|b| format!("0x{:02X}", b)).collect::<Vec<_>>());
            },
            _ => panic!("Expected MemorySequence, got {:?}", command.condition),
        }
    }

    #[test]
    fn test_registers_set_value16_three_digits() {
        let input = "registers set CP=0xFFF"; // Test three digit hex
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command,
                RegisterCommand::Set { assignment }
                if matches!(assignment.destination, RegisterSource::CommandPointer)
                && matches!(assignment.source, Source::Value(d) if d == 0x0FFF)
            )
        );
    }
}

pub struct RunCommandParser<'a> {
    context: &'a ParserContext<'a>,
}

impl<'a> RunCommandParser<'a> {
    pub fn new(context: &'a ParserContext<'a>) -> Self {
        Self { context }
    }

    pub fn from_pairs(pairs: Pairs<'_, Rule>, context: &'a ParserContext<'a>) -> AppResult<RunCommand> {
        let parser = Self::new(context);
        parser.parse_pairs(pairs)
    }

    fn parse_pairs(&self, pairs: Pairs<'_, Rule>) -> AppResult<RunCommand> {
        let mut start_address = None;
        let mut stop_condition = BooleanExpression::Value(true);
        let mut continue_condition = BooleanExpression::Value(true);

        for pair in pairs {
            match pair.as_rule() {
                Rule::run_address => {
                    if pair.as_str() == "init" {
                        start_address = Some(RunAddress::InitVector);
                    } else {
                        let addr_pair = pair.into_inner().next().unwrap();
                        start_address = Some(RunAddress::Memory(self.context.parse_memory(&addr_pair)?));
                    };
                }
                Rule::run_until_condition => {
                    stop_condition = self.context.parse_boolean_condition(pair.into_inner().next().unwrap().into_inner())?;
                }
                Rule::run_while_condition => {
                    // For while conditions:
                    // - Set continue_condition to the actual condition
                    // - Set stop_condition to false (only stop on continue check or infinite loop)
                    continue_condition = self.context.parse_boolean_condition(pair.into_inner().next().unwrap().into_inner())?;
                    stop_condition = BooleanExpression::Value(false);
                }
                stmt => panic!("unknown node type {stmt:?}. Is the Pest grammar up to date?"),
            }
        }

        Ok(RunCommand {
            stop_condition,
            continue_condition,
            start_address,
        })
    }
}

#[cfg(test)]
mod run_command_parser_tests {
    use super::*;
    use super::test_utils::setup_test_symbols;
    use crate::until_condition::{RegisterSource, Source};

    fn create_test_context<'a>() -> ParserContext<'a> {
        ParserContext::new(None)
    }

    #[test]
    fn simple_run() {
        let input = "run";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command.stop_condition, BooleanExpression::Value(v) if v));
        assert!(command.start_address.is_none());
    }

    #[test]
    fn run_with_start_address() {
        let input = "run #0x1234";
        let context = create_test_context();
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner(), &context).unwrap();

        assert!(matches!(command.stop_condition, BooleanExpression::Value(v) if v));
        assert!(matches!(command.start_address, Some(RunAddress::Memory(addr)) if addr == 0x1234));
    }

    #[test]
    fn run_with_stop_condition() {
        let input = "run until A > 0x12";
        let context = create_test_context();
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner(), &context).unwrap();

        if let BooleanExpression::StrictlyGreater(lt, rt) = command.stop_condition {
            assert!(matches!(lt, Source::Register(RegisterSource::Accumulator)));
            assert!(matches!(rt, Source::Value(data) if data == 0x12));
        } else {
            panic!(
                "Expected StrictlyGreater boolean expression, got '{:?}'.",
                command.stop_condition
            );
        }
        assert!(command.start_address.is_none());
    }

    #[test]
    fn run_init() {
        let input = "run init";
        let context = create_test_context();
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner(), &context).unwrap();

        assert!(matches!(
            command.start_address,
            Some(RunAddress::InitVector)
        ));
    }

    #[test]
    fn test_run_from_symbol_address_until_memory_at_symbol_matches() {
        let symbols = setup_test_symbols();
        let context = ParserContext::new(Some(&symbols));
        let input = "run $word_var until $counter = 0xff";
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command.start_address, Some(RunAddress::Memory(addr)) if addr == 0x1234));
        assert!(
            matches!(command.stop_condition, 
                BooleanExpression::Equal(
                    Source::Memory(addr),
                    Source::Value(0xff)
                ) if addr == 0x2000
            )
        );
    }

    #[test]
    fn test_run_until_cycle_count() {
        let input = "run until cycle_count > 0x0200";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        if let BooleanExpression::StrictlyGreater(lt, rt) = command.stop_condition {
            assert!(matches!(lt, Source::Register(RegisterSource::CycleCount)));
            assert!(matches!(rt, Source::Value(data) if data == 0x0200));
        } else {
            panic!(
                "Expected StrictlyGreater boolean expression, got '{:?}'.",
                command.stop_condition
            );
        }
        assert!(command.start_address.is_none());
    }

    #[test]
    fn test_run_with_cycle_count_comparison() {
        let input = "run #0x1234 until cycle_count = 0x0100";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(matches!(command.start_address, Some(RunAddress::Memory(addr)) if addr == 0x1234));
        if let BooleanExpression::Equal(lt, rt) = command.stop_condition {
            assert!(matches!(lt, Source::Register(RegisterSource::CycleCount)));
            assert!(matches!(rt, Source::Value(data) if data == 0x0100));
        } else {
            panic!(
                "Expected Equal boolean expression, got '{:?}'.",
                command.stop_condition
            );
        }
    }

    #[test]
    fn test_run_with_while_condition() {
        let input = "run while X = 0x42";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        // For while conditions:
        // - continue_condition should be the actual condition
        // - stop_condition should be Value(false) to only stop on continue check or infinite loop
        assert!(matches!(command.stop_condition, BooleanExpression::Value(false)));
        assert!(matches!(command.continue_condition, 
            BooleanExpression::Equal(
                Source::Register(RegisterSource::RegisterX),
                Source::Value(0x42)
            )
        ));
    }

    #[test]
    fn test_run_with_until_condition() {
        let input = "run until X = 0x42";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        // For until conditions:
        // - continue_condition should be Value(true) (no pre-check needed)
        // - stop_condition should be the actual condition
        assert!(matches!(command.continue_condition, BooleanExpression::Value(true)));
        assert!(matches!(command.stop_condition,
            BooleanExpression::Equal(
                Source::Register(RegisterSource::RegisterX),
                Source::Value(0x42)
            )
        ));
    }

    #[test]
    fn test_run_with_while_then_until() {
        let context = create_test_context();
        
        // First parse a while condition
        let while_input = "run while X = 0x42";
        let while_command = RunCommandParser::from_pairs(
            PestParser::parse(Rule::run_instruction, while_input)
                .unwrap()
                .next()
                .unwrap()
                .into_inner(),
            &context
        ).unwrap();

        // Verify while condition setup
        assert!(matches!(while_command.stop_condition, BooleanExpression::Value(false)));
        assert!(matches!(while_command.continue_condition,
            BooleanExpression::Equal(
                Source::Register(RegisterSource::RegisterX),
                Source::Value(0x42)
            )
        ));

        // Then parse an until condition
        let until_input = "run until Y = 0x24";
        let until_command = RunCommandParser::from_pairs(
            PestParser::parse(Rule::run_instruction, until_input)
                .unwrap()
                .next()
                .unwrap()
                .into_inner(),
            &context
        ).unwrap();

        // Verify until condition setup
        assert!(matches!(until_command.continue_condition, BooleanExpression::Value(true)));
        assert!(matches!(until_command.stop_condition,
            BooleanExpression::Equal(
                Source::Register(RegisterSource::RegisterY),
                Source::Value(0x24)
            )
        ));
    }

    #[test]
    fn test_run_with_complex_while_conditions() {
        let test_cases = [
            // Simple register comparison
            ("run while A < 0x80", 
                Box::new(|cond: &BooleanExpression| matches!(cond, 
                    BooleanExpression::StrictlyLesser(
                        Source::Register(RegisterSource::Accumulator),
                        Source::Value(0x80)
                    )
                )) as Box<dyn Fn(&BooleanExpression) -> bool>
            ),
            
            // Complex conditions
            ("run while X >= 0x10",
                Box::new(|cond: &BooleanExpression| matches!(cond,
                    BooleanExpression::GreaterOrEqual(
                        Source::Register(RegisterSource::RegisterX),
                        Source::Value(0x10)
                    )
                ))
            ),
            
            // With cycle count
            ("run while cycle_count < 0x1000",
                Box::new(|cond: &BooleanExpression| matches!(cond,
                    BooleanExpression::StrictlyLesser(
                        Source::Register(RegisterSource::CycleCount),
                        Source::Value(0x1000)
                    )
                ))
            ),
        ];

        let context = create_test_context();

        for (input, matcher) in test_cases {
            let pairs = PestParser::parse(Rule::run_instruction, input)
                .unwrap()
                .next()
                .unwrap()
                .into_inner();
            let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

            // Verify stop_condition is false
            assert!(matches!(command.stop_condition, BooleanExpression::Value(false)));
            
            // Verify continue_condition matches expected pattern
            assert!(matcher(&command.continue_condition));
            
            assert!(command.start_address.is_none());
        }
    }

    #[test]
    fn test_run_with_while_and_start_address() {
        let input = "run #0x1234 while A = 0x42";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        // Verify start address
        assert!(matches!(command.start_address, Some(RunAddress::Memory(addr)) if addr == 0x1234));

        // Verify stop_condition is false
        assert!(matches!(command.stop_condition, BooleanExpression::Value(false)));

        // Verify continue_condition
        assert!(matches!(command.continue_condition,
            BooleanExpression::Equal(
                Source::Register(RegisterSource::Accumulator),
                Source::Value(0x42)
            )
        ));
    }

    #[test]
    fn test_run_with_while_and_symbol() {
        let symbols = test_utils::setup_test_symbols();
        let context = ParserContext::new(Some(&symbols));
        let input = "run while $byte_var = 0x42";
        
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs, &context).unwrap();

        // Verify stop_condition is false
        assert!(matches!(command.stop_condition, BooleanExpression::Value(false)));

        // Verify continue_condition with symbol
        assert!(matches!(command.continue_condition,
            BooleanExpression::Equal(
                Source::Memory(addr),
                Source::Value(0x42)
            ) if addr == 0x34  // 0x34 is byte_var's address
        ));
    }
}

pub struct AssertCommandParser<'a> {
    context: &'a ParserContext<'a>,
}

impl<'a> AssertCommandParser<'a> {
    pub fn new(context: &'a ParserContext<'a>) -> Self {
        Self { context }
    }

    pub fn from_pairs(pairs: Pairs<'_, Rule>, context: &'a ParserContext<'a>) -> AppResult<AssertCommand> {
        let parser = Self::new(context);
        parser.parse_pairs(pairs)
    }

    fn parse_pairs(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<AssertCommand> {
        let boolean_condition = pairs.next().unwrap();
        let condition = match boolean_condition.as_rule() {
            Rule::boolean_condition => {
                let first = boolean_condition.into_inner().next().unwrap();
                match first.as_rule() {
                    Rule::memory_sequence => self.parse_memory_sequence(first)?,
                    Rule::pointer_assertion => self.parse_pointer_assertion(first)?,
                    _ => self.context.parse_boolean_condition(Pairs::single(first))?,
                }
            }
            _ => panic!("Expected boolean_condition, got {:?}", boolean_condition.as_rule()),
        };

        let comment = pairs.next().unwrap().as_str().to_string();
        let command = AssertCommand { comment, condition };

        Ok(command)
    }

    fn parse_pointer_assertion(&self, node: Pair<Rule>) -> AppResult<BooleanExpression> {
        let mut nodes = node.into_inner();
        
        // Get the pointer location (memory address where the pointer is stored)
        let pointer_loc = nodes.next().expect("pointer_assertion should have a memory address");
        let pointer_addr = self.context.parse_memory(&pointer_loc)?;

        // Skip the "points" and "to" tokens by getting the pointer_target
        let target_node = nodes.next().expect("pointer_assertion should have a pointer_target");
        let mut target_nodes = target_node.into_inner();
        
        // Get the base target address
        let target_addr_node = target_nodes.next().expect("pointer_target should have a memory address");
        let mut target_addr = self.context.parse_memory(&target_addr_node)?;

        // Check for optional offset
        if let Some(offset_node) = target_nodes.next() {
            let mut offset_nodes = offset_node.into_inner();
            let op_node = offset_nodes.next().unwrap();
            let value_node = offset_nodes.next().unwrap();
            
            let offset = match value_node.as_rule() {
                Rule::value8 | Rule::value16 => self.context.parse_source_value(&value_node)?,
                _ => panic!("Unexpected offset type: {:?}", value_node.as_rule()),
            };
            
            if let Source::Value(offset_val) = offset {
                target_addr = match op_node.as_rule() {
                    Rule::plus_op => target_addr.wrapping_add(offset_val),
                    Rule::minus_op => target_addr.wrapping_sub(offset_val),
                    _ => panic!("Unexpected operator type: {:?}", op_node.as_rule()),
                };
            }
        }

        // For 6502, pointers are stored in little-endian format
        // So we need to check that pointer_addr contains the low byte
        // and pointer_addr + 1 contains the high byte
        let low_byte = target_addr & 0xFF;
        let high_byte = (target_addr >> 8) & 0xFF;

        // Create a boolean expression that checks both bytes
        let low_check = BooleanExpression::Equal(
            Source::Memory(pointer_addr),
            Source::Value(low_byte)
        );
        let high_check = BooleanExpression::Equal(
            Source::Memory(pointer_addr.wrapping_add(1)),
            Source::Value(high_byte)
        );

        // Combine the checks with AND
        Ok(BooleanExpression::And(
            Box::new(low_check),
            Box::new(high_check)
        ))
    }

    fn parse_memory_sequence(&self, node: Pair<Rule>) -> AppResult<BooleanExpression> {
        let mut seq_nodes = node.into_inner();
        let addr_node = seq_nodes.next().expect("memory_sequence should have a memory_location node");
        let addr = self.context.parse_source_memory(&addr_node)?;
        
        let sequence_node = seq_nodes.next().expect("memory_sequence should have a bytes_list or string_literal node");
        let bytes = match sequence_node.as_rule() {
            Rule::bytes_list => {
                let bytes_node = sequence_node.into_inner().next().expect("bytes_list should contain a bytes node");
                self.context.parse_hex_sequence(bytes_node.as_str())?
            }
            Rule::string_literal => {
                // Remove the quotes
                let str_content = &sequence_node.as_str()[1..sequence_node.as_str().len()-1];
                self.context.parse_string_literal(str_content)
            }
            _ => panic!("Expected bytes_list or string_literal in memory_sequence")
        };
        
        Ok(BooleanExpression::MemorySequence(addr, bytes))
    }
}

#[cfg(test)]
mod assert_parser_tests {
    use super::*;
    use super::test_utils::setup_test_symbols;

    fn create_test_context<'a>() -> ParserContext<'a> {
        ParserContext::new(None)
    }

    #[test]
    fn test_assert_parser() {
        let input = "assert A = 0x00 $$something$$";
        let context = create_test_context();
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .expect("one instruction per line")
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command, AssertCommand { comment, condition } if comment.as_str() == "something" && matches!(condition, BooleanExpression::Equal(_, _)))
        )
    }

    #[test]
    fn test_assert_command_with_symbols() {
        let symbols = setup_test_symbols();
        let context = ParserContext::new(Some(&symbols));
        let input = "assert $byte_var = 0xff $$test description$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "test description");
        assert!(
            matches!(command.condition,
                BooleanExpression::Equal(
                    Source::Memory(addr),
                    Source::Value(0xff)
                ) if addr == 0x34
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence() {
        let context = create_test_context();
        let input = "assert #0x8000 ~ 0x(01,a2,f3) $$check memory sequence$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check memory sequence");
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == vec![0x01, 0xa2, 0xf3]
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence_with_symbols() {
        let mut symbols = setup_test_symbols();
        symbols.add_symbol(0x8000, "code_start".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "assert $code_start ~ 0x(01,a2,f3) $$check memory sequence with symbol$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check memory sequence with symbol");
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == vec![0x01, 0xa2, 0xf3]
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence_invalid_source() {
        // Try to use a register with the sequence operator - should be rejected by grammar
        let input = "assert A ~ 0x(01,02) $$invalid - register with sequence$$";
        assert!(PestParser::parse(Rule::assert_instruction, input).is_err());

        // Verify that memory addresses still work
        let input = "assert #0x1234 ~ 0x(01,02) $$valid - memory with sequence$$";
        assert!(PestParser::parse(Rule::assert_instruction, input).is_ok());
    }

    #[test]
    fn test_assert_memory_sequence_with_string() {
        let context = create_test_context();
        let input = "assert #0x8000 ~ \"hello\" $$check memory sequence with string$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check memory sequence with string");
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == b"hello".to_vec()
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence_with_hex_escape_sequences() {
        let context = create_test_context();
        
        // Test basic hex escape sequences in assertions
        let input = "assert #0x8000 ~ \"\\xFF\\x00\\x42\" $$check memory sequence with hex escapes$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check memory sequence with hex escapes");
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == vec![0xFF, 0x00, 0x42]
            )
        );

        // Test mixed text and hex escape sequences in assertions
        let input = "assert #0x8000 ~ \"data:\\x0A\\xFF\\x00end\" $$check mixed string with hex escapes$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check mixed string with hex escapes");
        let expected = b"data:\x0A\xFF\x00end".to_vec();
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == expected
            )
        );

        // Test mixing hex escapes with standard escapes
        let input = "assert #0x8000 ~ \"\\x41\\n\\t\\xFF\\0\" $$check mixed escape sequences$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check mixed escape sequences");
        let expected = b"A\n\t\xFF\0".to_vec();
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == expected
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence_with_line_continuation() {
        let context = create_test_context();
        
        // Test basic line continuation in assertions
        let input = "assert #0x8000 ~ \"hello\\\nworld\" $$check memory sequence with line continuation$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check memory sequence with line continuation");
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == b"helloworld".to_vec()
            )
        );

        // Test multi-line screen layout assertion
        let input = "assert #0x8000 ~ \"+-------+\\\n| Hello |\\\n+-------+\" $$check screen layout$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check screen layout");
        let expected = b"+-------+| Hello |+-------+".to_vec();
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == expected
            )
        );

        // Test mixing line continuation with escape sequences
        let input = "assert #0x8000 ~ \"start\\\n\\t\\xFF\\\nend\" $$check mixed line continuation and escapes$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "check mixed line continuation and escapes");
        let expected = b"start\t\xFFend".to_vec();
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x8000 && bytes == expected
            )
        );
    }

    #[test]
    fn test_pointer_assertion() {
        // Create context with necessary symbols
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "entry_loc".to_string());
        symbols.add_symbol(0x2000, "cache".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test basic pointer assertion
        let input = "assert $entry_loc -> $cache $$basic pointer test$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "basic pointer test");
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0x00);  // Low byte of 0x2000
                    assert_eq!(high_val, 0x20); // High byte of 0x2000
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
        
        // Test pointer assertion with offset
        let input = "assert $entry_loc -> $cache + 0x20 $$pointer with offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "pointer with offset");
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0x20);  // Low byte of 0x2020
                    assert_eq!(high_val, 0x20); // High byte of 0x2020
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
        
        // Test pointer assertion with decimal offset
        let input = "assert $entry_loc -> $cache + 32 $$pointer with decimal offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "pointer with decimal offset");
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0x20);  // Low byte of 0x2020 (32 = 0x20)
                    assert_eq!(high_val, 0x20); // High byte of 0x2020
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
    }

    #[test]
    fn test_pointer_assertion_with_symbols() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "entry_loc".to_string());
        symbols.add_symbol(0x2000, "cache".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "assert $entry_loc -> $cache + 0x20 $$pointer test with symbols$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        // The pointer at $entry_loc (0x1000) should point to $cache (0x2000) + 0x20 = 0x2020
        // So memory[0x1000] should contain 0x20 (low byte)
        // and memory[0x1001] should contain 0x20 (high byte)
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0x20);  // Low byte of 0x2020
                    assert_eq!(high_val, 0x20); // High byte of 0x2020
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
    }

    #[test]
    fn test_pointer_assertion_wrapping() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "ptr".to_string());
        symbols.add_symbol(0xFFE0, "near_end".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test wrapping behavior when adding offset
        let input = "assert $ptr -> $near_end + 0x30 $$pointer should wrap$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        // 0xFFE0 + 0x30 = 0x10 (after wrapping)
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0x10);  // Low byte after wrapping
                    assert_eq!(high_val, 0x00); // High byte after wrapping
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
    }

    #[test]
    fn test_pointer_assertion_with_negative_offset() {
        // Create context with necessary symbols
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "ptr".to_string());
        symbols.add_symbol(0x2000, "base".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test negative offset
        let input = "assert $ptr -> $base - 0x20 $$pointer with negative offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "pointer with negative offset");
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0xE0);  // Low byte of 0x1FE0 (0x2000 - 0x20)
                    assert_eq!(high_val, 0x1F); // High byte of 0x1FE0
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
    }

    #[test]
    fn test_pointer_assertion_wrapping_with_negative_offset() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "ptr".to_string());
        symbols.add_symbol(0x0020, "near_start".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test wrapping behavior when subtracting offset
        let input = "assert $ptr -> $near_start - 0x30 $$pointer should wrap$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        // 0x0020 - 0x30 = 0xFFF0 (after wrapping)
        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0xF0);  // Low byte after wrapping
                    assert_eq!(high_val, 0xFF); // High byte after wrapping
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
    }

    #[test]
    fn test_pointer_assertion_with_decimal_negative_offset() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "ptr".to_string());
        symbols.add_symbol(0x2000, "base".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test negative decimal offset
        let input = "assert $ptr -> $base - 32 $$pointer with decimal negative offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        if let BooleanExpression::And(low_check, high_check) = command.condition {
            match (*low_check, *high_check) {
                (
                    BooleanExpression::Equal(Source::Memory(low_addr), Source::Value(low_val)),
                    BooleanExpression::Equal(Source::Memory(high_addr), Source::Value(high_val))
                ) => {
                    assert_eq!(low_addr, 0x1000);
                    assert_eq!(high_addr, 0x1001);
                    assert_eq!(low_val, 0xE0);  // Low byte of 0x1FE0 (0x2000 - 32)
                    assert_eq!(high_val, 0x1F); // High byte of 0x1FE0
                },
                _ => panic!("Unexpected boolean expression structure"),
            }
        } else {
            panic!("Expected And expression at top level");
        }
    }

    #[test]
    fn test_assert_with_memory_offset() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x2000, "cache".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test positive offset with comparison
        let input = "assert $cache + 1 = 0x00 $$test memory offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "test memory offset");
        assert!(
            matches!(command.condition,
                BooleanExpression::Equal(
                    Source::Memory(addr),
                    Source::Value(0x00)
                ) if addr == 0x2001
            )
        );

        // Test negative offset with comparison
        let input = "assert $cache - 1 = 0x42 $$test negative memory offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command.condition,
                BooleanExpression::Equal(
                    Source::Memory(addr),
                    Source::Value(0x42)
                ) if addr == 0x1FFF
            )
        );
    }

    #[test]
    fn test_assert_memory_sequence_with_offset() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x2000, "cache".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test memory sequence with positive offset
        let input = "assert $cache + 2 ~ 0x(00,40) $$test sequence with offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert_eq!(command.comment, "test sequence with offset");
        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x2002 && bytes == vec![0x00, 0x40]
            )
        );

        // Test memory sequence with decimal offset
        let input = "assert $cache + 128 ~ 0x(00,40) $$test sequence with decimal offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        assert!(
            matches!(command.condition,
                BooleanExpression::MemorySequence(
                    Source::Memory(addr),
                    bytes
                ) if addr == 0x2080 && bytes == vec![0x00, 0x40]
            )
        );
    }

    #[test]
    fn test_memory_offset_wrapping() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0xFFFF, "end".to_string());
        symbols.add_symbol(0x0000, "start".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // Test wrapping with positive offset
        let input = "assert $end + 2 = 0x42 $$test wrapping with positive offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        // 0xFFFF + 2 should wrap around to 0x0001
        if let BooleanExpression::Equal(Source::Memory(addr), Source::Value(val)) = command.condition {
            assert_eq!(addr, 0x0001, "Expected wrapped address 0x0001, got 0x{:04X}", addr);
            assert_eq!(val, 0x42, "Expected value 0x42, got 0x{:02X}", val);
        } else {
            panic!("Expected Equal expression, got {:?}", command.condition);
        }

        // Test wrapping with negative offset
        let input = "assert $start - 2 = 0x42 $$test wrapping with negative offset$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        // 0x0000 - 2 should wrap around to 0xFFFE
        if let BooleanExpression::Equal(Source::Memory(addr), Source::Value(val)) = command.condition {
            assert_eq!(addr, 0xFFFE, "Expected wrapped address 0xFFFE, got 0x{:04X}", addr);
            assert_eq!(val, 0x42, "Expected value 0x42, got 0x{:02X}", val);
        } else {
            panic!("Expected Equal expression, got {:?}", command.condition);
        }
    }

    #[test]
    fn test_assert_with_symbol_byte_references() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1234, "test_str1".to_string());
        let context = ParserContext::new(Some(&symbols));

        // Test low byte reference with register
        let pairs = PestParser::parse(Rule::assert_instruction, "assert A = <$test_str1 $$low byte test$$")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();

        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        if let BooleanExpression::Equal(lh, rh) = command.condition {
            assert_eq!(lh, Source::Register(RegisterSource::Accumulator));
            assert_eq!(rh, Source::Value(0x34)); // Low byte of 0x1234
        } else {
            panic!("Expected Equal assertion with low byte");
        }

        // Test high byte reference with register
        let pairs = PestParser::parse(Rule::assert_instruction, "assert X = >$test_str1 $$high byte test$$")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();

        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        if let BooleanExpression::Equal(lh, rh) = command.condition {
            assert_eq!(lh, Source::Register(RegisterSource::RegisterX));
            assert_eq!(rh, Source::Value(0x12)); // High byte of 0x1234
        } else {
            panic!("Expected Equal assertion with high byte");
        }
    }

    #[test]
    fn test_assert_with_symbol_byte_references_memory_comparison() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1234, "test_str1".to_string());
        symbols.add_symbol(0x2000, "cps_strings_a".to_string());
        symbols.add_symbol(0x2001, "cps_strings_x".to_string());
        let context = ParserContext::new(Some(&symbols));

        // Test the original user example for low byte
        let pairs = PestParser::parse(Rule::assert_instruction, "assert $cps_strings_a = <$test_str1 $$t00: cputs called with correct pointer (low byte)$$")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();

        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        if let BooleanExpression::Equal(lh, rh) = command.condition {
            assert_eq!(lh, Source::Memory(0x2000));
            assert_eq!(rh, Source::Value(0x34)); // Low byte of 0x1234
        } else {
            panic!("Expected Equal assertion with memory and low byte");
        }

        // Test the original user example for high byte
        let pairs = PestParser::parse(Rule::assert_instruction, "assert $cps_strings_x = >$test_str1 $$t00: cputs called with correct pointer (high byte)$$")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();

        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();

        if let BooleanExpression::Equal(lh, rh) = command.condition {
            assert_eq!(lh, Source::Memory(0x2001));
            assert_eq!(rh, Source::Value(0x12)); // High byte of 0x1234
        } else {
            panic!("Expected Equal assertion with memory and high byte");
        }
    }

    #[test]
    fn test_symbol_byte_references_with_missing_symbol() {
        let context = ParserContext::new(None);

        // Test that missing symbol table produces error
        let pairs = PestParser::parse(Rule::assert_instruction, "assert A = <$missing_symbol $$error test$$")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();

        let result = AssertCommandParser::from_pairs(pairs, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Symbol table not available"));

        // Test that missing symbol produces error
        let symbols = test_utils::setup_test_symbols();
        let context = ParserContext::new(Some(&symbols));

        let pairs = PestParser::parse(Rule::assert_instruction, "assert A = >$nonexistent $$error test$$")
            .unwrap()
            .next()
            .unwrap()
            .into_inner();

        let result = AssertCommandParser::from_pairs(pairs, &context);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Symbol 'nonexistent' not found"));
    }
}

pub struct CliCommandParser<'a> {
    context: ParserContext<'a>,
}

impl<'a> CliCommandParser<'a> {
    // Static method for parsing without symbols (mainly used in tests)
    pub fn from(line: &str) -> AppResult<CliCommand> {
        Self::from_with_context(line, ParserContext::new(None))
    }

    // Main method used in production code, takes a context with symbols
    pub fn from_with_context(line: &str, context: ParserContext<'a>) -> AppResult<CliCommand> {
        let parser = Self { context };
        parser.parse_line(line)
    }

    fn parse_line(&self, line: &str) -> AppResult<CliCommand> {
        let line = line.trim();

        if line.is_empty() {
            return Ok(CliCommand::None);
        }

        let pair = PestParser::parse(Rule::sentence, line)?
            .next()
            .expect("There is only one sentence per input.");

        // comments are ignored
        if pair.as_rule() == Rule::EOI {
            return Ok(CliCommand::None);
        }

        let pair = pair
            .into_inner()
            .next()
            .expect("There is only one instruction per sentence.");

        let command = match pair.as_rule() {
            Rule::run_instruction => {
                CliCommand::Run(RunCommandParser::from_pairs(pair.into_inner(), &self.context)?)
            }
            Rule::assert_instruction => {
                CliCommand::Assert(AssertCommandParser::from_pairs(pair.into_inner(), &self.context)?)
            }
            Rule::marker => {
                let marker = pair.into_inner().next().unwrap().as_str();
                CliCommand::Marker(marker.to_owned())
            }
            Rule::registers_instruction => {
                CliCommand::Registers(RegisterCommandParser::from_pairs(pair.into_inner(), &self.context)?)
            }
            Rule::memory_instruction => {
                CliCommand::Memory(MemoryCommandParser::from_pairs(pair.into_inner(), &self.context)?)
            }
            Rule::symbols_instruction => {
                CliCommand::Memory(MemoryCommandParser::from_pairs(pair.into_inner(), &self.context)?)
            }
            Rule::disassemble_instruction => {
                let mut pairs = pair.into_inner();
                let start = self.context.parse_memory(&pairs.next().unwrap())?;
                let length_node = pairs.next().unwrap();
                let length_str = &length_node.as_str()[2..]; // Skip the "0x" prefix
                let length = usize::from_str_radix(length_str, 16)
                    .map_err(|e| anyhow::anyhow!("Invalid hex length {}: {}", length_str, e))?;
                if length == 0 {
                    return Err(anyhow::anyhow!("Length must be greater than 0"));
                }
                CliCommand::Disassemble { 
                    start, 
                    end: start + length - 1 
                }
            }
            Rule::enable_instruction => {
                let mut pairs = pair.into_inner();
                let function_name = pairs.next().unwrap().as_str();
                let function = match function_name {
                    "trace_logging" => ControllableFunction::TraceLogging,
                    _ => return Err(anyhow::anyhow!("Unknown function: {}", function_name)),
                };
                CliCommand::Enable(function)
            }
            Rule::disable_instruction => {
                let mut pairs = pair.into_inner();
                let function_name = pairs.next().unwrap().as_str();
                let function = match function_name {
                    "trace_logging" => ControllableFunction::TraceLogging,
                    _ => return Err(anyhow::anyhow!("Unknown function: {}", function_name)),
                };
                CliCommand::Disable(function)
            }
            _ => {
                panic!(
                    "'{}' was not expected here: 'register|memory|run|assert|reset|symbols|disassemble|enable|disable instruction'.",
                    pair.as_str()
                );
            }
        };

        Ok(command)
    }
}

#[cfg(test)]
mod cli_command_parser_test {
    use super::*;

    #[test]
    fn test_empty_input() {
        let cli_command = CliCommandParser::from("").unwrap();
        assert!(matches!(cli_command, CliCommand::None));

        let cli_command = CliCommandParser::from("      ").unwrap();
        assert!(matches!(cli_command, CliCommand::None));
    }

    #[test]
    fn test_run_cli_parser() {
        let cli_command = CliCommandParser::from("run #0x1aff until X = 0xff").unwrap();
        assert!(matches!(cli_command, CliCommand::Run(_)));
    }

    #[test]
    fn test_assert_cli_parser() {
        let cli_command = CliCommandParser::from("assert #0x0000=0x00 $$description$$").unwrap();
        assert!(matches!(cli_command, CliCommand::Assert(_)));
    }

    #[test]
    fn test_marker_cli_parser() {
        let cli_command = CliCommandParser::from("marker $$This is a marker.$$").unwrap();
        assert!(
            matches!(cli_command, CliCommand::Marker(comment) if comment == *"This is a marker.")
        );
    }

    #[test]
    fn test_registers_cli_parser() {
        let cli_command = CliCommandParser::from("registers flush").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Registers(RegisterCommand::Flush)
        ));
    }

    #[test]
    fn test_memory_cli_flush_parser() {
        let cli_command = CliCommandParser::from("memory flush").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::Flush)
        ));
    }

    #[test]
    fn test_memory_write_parser() {
        let cli_command = CliCommandParser::from("memory write #0x1234 0x(12,23,34,45)").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::Write {
                address,
                bytes
            }) if address == 0x1234 && bytes == vec![0x12, 0x23, 0x34, 0x45]
        ));
    }

    #[test]
    fn test_memory_load_parser() {
        let cli_command = CliCommandParser::from("memory load #0x1234 \"file.test\"").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::Load {
                address,
                filepath
            }) if address == 0x1234 && filepath == PathBuf::from("file.test")
        ));
    }

    #[test]
    fn test_code_comments() {
        let cli_command = CliCommandParser::from("// This is a comment").unwrap();
        assert!(matches!(cli_command, CliCommand::None));
        
        let cli_command = CliCommandParser::from("; This is also a comment").unwrap();
        assert!(matches!(cli_command, CliCommand::None));
    }

    #[test]
    fn test_memory_load_parser_with_symbols() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x1234, "test_var".to_string());

        let cli_command = CliCommandParser::from_with_context(
            "memory write $test_var 0x(12)",
            ParserContext::new(Some(&symbols))
        ).unwrap();

        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::Write {
                address,
                bytes
            }) if address == 0x1234 && bytes == vec![0x12]
        ));
    }

    #[test]
    fn test_symbols_load_parser() {
        let cli_command = CliCommandParser::from("symbols load \"tests/symbols.txt\"").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::LoadSymbols { symbols: _ })
        ));
    }

    #[test]
    fn test_symbols_add_parser() {
        let cli_command = CliCommandParser::from("symbols add foo=0x1234").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Memory(MemoryCommand::AddSymbol { name, value })
            if name == "foo" && value == 0x1234
        ));
    }

    #[test]
    fn test_disassemble_parser() {
        // Basic hex address and length parsing
        let cli_command = CliCommandParser::from("disassemble #0x1000 0x10").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Disassemble { start, end }
            if start == 0x1000 && end == 0x100F
        ));

        // With symbols for start address
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x1234, "code_start".to_string());
        let cli_command = CliCommandParser::from_with_context(
            "disassemble $code_start 0x100",
            ParserContext::new(Some(&symbols))
        ).unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Disassemble { start, end }
            if start == 0x1234 && end == 0x1333
        ));

        // Test various length formats
        let cli_command = CliCommandParser::from("disassemble #0x1000 0x1").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Disassemble { start, end }
            if start == 0x1000 && end == 0x1000
        ));

        let cli_command = CliCommandParser::from("disassemble #0x1000 0x0f").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Disassemble { start, end }
            if start == 0x1000 && end == 0x100E
        ));

        // Error cases
        assert!(CliCommandParser::from("disassemble").is_err()); // Missing parameters
        assert!(CliCommandParser::from("disassemble #0x1000").is_err()); // Missing length
        assert!(CliCommandParser::from("disassemble #0xZZZZ 0x10").is_err()); // Invalid hex address
        assert!(CliCommandParser::from("disassemble #0x1000 0xZZZZ").is_err()); // Invalid hex length
    }

    #[test]
    fn test_enable_disable_parser() {
        // Test enable trace_logging
        let cli_command = CliCommandParser::from("enable trace_logging").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Enable(ControllableFunction::TraceLogging)
        ));

        // Test disable trace_logging
        let cli_command = CliCommandParser::from("disable trace_logging").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Disable(ControllableFunction::TraceLogging)
        ));

        // Test case insensitivity for command words
        let cli_command = CliCommandParser::from("ENABLE trace_logging").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Enable(ControllableFunction::TraceLogging)
        ));

        let cli_command = CliCommandParser::from("DISABLE trace_logging").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Disable(ControllableFunction::TraceLogging)
        ));

        // Test mixed case
        let cli_command = CliCommandParser::from("Enable trace_logging").unwrap();
        assert!(matches!(
            cli_command,
            CliCommand::Enable(ControllableFunction::TraceLogging)
        ));

        // Test error cases
        assert!(CliCommandParser::from("enable").is_err()); // Missing function name
        assert!(CliCommandParser::from("disable").is_err()); // Missing function name
        assert!(CliCommandParser::from("enable unknown_function").is_err()); // Unknown function
        assert!(CliCommandParser::from("disable unknown_function").is_err()); // Unknown function
    }
}

#[cfg(test)]
mod parser_context_tests {
    use super::*;
    use super::test_utils::setup_test_symbols;
    use crate::until_condition::{BooleanExpression, RegisterSource, Source};

    fn create_test_context<'a>() -> ParserContext<'a> {
        ParserContext::new(None)
    }

    #[test]
    fn test_parse_boolean_condition() {
        let input = "A != 0xff";
        let context = create_test_context();
        let node = PestParser::parse(Rule::boolean_condition, input)
            .unwrap()
            .next()
            .expect("There is one node in this input.");
        let output = context.parse_boolean_condition(node.into_inner()).unwrap();

        assert!(matches!(
            output,
            BooleanExpression::Different(
                Source::Register(RegisterSource::Accumulator),
                Source::Value(0xff)
            )
        ));
    }

    #[test]
    fn test_parse_boolean_condition_memory_sequence() {
        let input = "#0x8000 ~ 0x(01,a2,f3)";
        let context = create_test_context();
        let node = PestParser::parse(Rule::boolean_condition, input)
            .unwrap()
            .next()
            .expect("There is one node in this input.");
        let output = context.parse_boolean_condition(node.into_inner()).unwrap();

        assert!(matches!(
            output,
            BooleanExpression::MemorySequence(
                Source::Memory(addr),
                bytes
            ) if addr == 0x8000 && bytes == vec![0x01, 0xa2, 0xf3]
        ));
    }

    #[test]
    fn test_parse_boolean_condition_memory_sequence_with_symbol() {
        let mut symbols = setup_test_symbols();
        symbols.add_symbol(0x8000, "code_start".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "$code_start ~ 0x(01,a2,f3)";
        let node = PestParser::parse(Rule::boolean_condition, input)
            .unwrap()
            .next()
            .expect("There is one node in this input.");
        let output = context.parse_boolean_condition(node.into_inner()).unwrap();

        assert!(matches!(
            output,
            BooleanExpression::MemorySequence(
                Source::Memory(addr),
                bytes
            ) if addr == 0x8000 && bytes == vec![0x01, 0xa2, 0xf3]
        ));
    }

    #[test]
    fn test_parse_hex_ok() {
        let context = create_test_context();
        assert_eq!(0x02ff, context.parse_hex("02ff").unwrap());
        assert_eq!(0x0000, context.parse_hex("0000").unwrap());
        assert_eq!(0xea, context.parse_hex("ea").unwrap());
    }

    #[test]
    fn test_parse_hex_bad() {
        let context = create_test_context();
        context.parse_hex("").expect_err("Empty string must yield an error.");
        context.parse_hex("   ").expect_err("Invisible string must yield an error.");
        context.parse_hex("xxx").expect_err("Non hexa must yield an error.");
    }

    #[test]
    fn test_parse_memory_with_hex_address() {
        let context = create_test_context();
        let input = "#0x1234";
        let node = PestParser::parse(Rule::hex_address, input)
            .unwrap()
            .next()
            .unwrap();
        let result = context.parse_memory(&node).unwrap();
        assert_eq!(0x1234, result);
    }

    #[test]
    fn test_parse_memory_with_symbol() {
        let mut symbols = SymbolTable::new();
        symbols.add_symbol(0x1234, "test_var".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        let input = "$test_var";
        let node = PestParser::parse(Rule::symbol_reference, input)
            .unwrap()
            .next()
            .unwrap();
        let result = context.parse_memory(&node).unwrap();
        assert_eq!(0x1234, result);
    }

    #[test]
    fn test_parse_memory_with_missing_symbol() {
        let context = create_test_context();
        let input = "$nonexistent";
        let node = PestParser::parse(Rule::symbol_reference, input)
            .unwrap()
            .next()
            .unwrap();
        context.parse_memory(&node).expect_err("Should fail when symbols table is not available");
    }

    #[test]
    fn test_run_with_complex_conditions() {
        let test_cases = [
            ("run while (A < 0x80 AND X = 0x42)", "while with AND"),
            ("run until (A = 0x42 OR X = 0x10)", "until with OR"),
            ("run while (A = 0x42 AND X = 0x10) OR Y = 0x20", "while with AND and OR"),
            ("run until ((A = 0x42) AND (X = 0x10))", "until with nested brackets"),
        ];

        let _context = create_test_context();

        for (input, test_name) in test_cases {
            let _pairs = PestParser::parse(Rule::run_instruction, input)
                .unwrap_or_else(|e| panic!("Failed to parse '{}' ({}): {}", input, test_name, e));
        }
    }

    #[test]
    fn test_boolean_expressions() {
        let test_cases = [
            // Simple comparisons
            ("A = 0x42", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Equal(
                    Source::Register(RegisterSource::Accumulator),
                    Source::Value(0x42)
                ))
            }) as Box<dyn Fn(&BooleanExpression) -> bool>),

            // Simple parenthesized expressions
            ("(A = 0x42)", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Equal(
                    Source::Register(RegisterSource::Accumulator),
                    Source::Value(0x42)
                ))
            })),
            
            // Nested parentheses
            ("((A = 0x42))", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Equal(
                    Source::Register(RegisterSource::Accumulator),
                    Source::Value(0x42)
                ))
            })),
            // Different comparison operators
            ("A >= 0x42", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::GreaterOrEqual(
                    Source::Register(RegisterSource::Accumulator),
                    Source::Value(0x42)
                ))
            })),
            ("X < 0x10", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::StrictlyLesser(
                    Source::Register(RegisterSource::RegisterX),
                    Source::Value(0x10)
                ))
            })),
            ("cycle_count != 0x100", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Different(
                    Source::Register(RegisterSource::CycleCount),
                    Source::Value(0x100)
                ))
            })),

            // Basic operators
            ("A = 0x42 AND X = 0x10", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::And(_, _))
            })),
            ("A = 0x42 OR X = 0x10", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Or(_, _))
            })),

            // Multiple AND operations (left associative)
            ("A = 0x42 AND X = 0x10 AND Y = 0x20", Box::new(|expr: &BooleanExpression| {
                if let BooleanExpression::And(left, _) = expr {
                    matches!(**left, BooleanExpression::And(_, _))
                } else {
                    false
                }
            })),
            // Multiple OR operations (left associative)
            ("A = 0x42 OR X = 0x10 OR Y = 0x20", Box::new(|expr: &BooleanExpression| {
                if let BooleanExpression::Or(left, _) = expr {
                    matches!(**left, BooleanExpression::Or(_, _))
                } else {
                    false
                }
            })),
            
            // Operator precedence (AND binds tighter than OR)
            ("A = 0x42 AND X < 0x10 OR Y > 0x20", Box::new(|expr: &BooleanExpression| {
                if let BooleanExpression::Or(left, right) = expr {
                    matches!(**left, BooleanExpression::And(_, _)) &&
                    matches!(**right, BooleanExpression::StrictlyGreater(
                        Source::Register(RegisterSource::RegisterY),
                        Source::Value(0x20)
                    ))
                } else {
                    false
                }
            })),
            
            // AND with parenthesized OR
            ("A = 0x42 AND (X < 0x10 OR Y > 0x20)", Box::new(|expr: &BooleanExpression| {
                if let BooleanExpression::And(left, right) = expr {
                    matches!(**left, BooleanExpression::Equal(
                        Source::Register(RegisterSource::Accumulator),
                        Source::Value(0x42)
                    )) &&
                    matches!(**right, BooleanExpression::Or(_, _))
                } else {
                    false
                }
            })),
            
            // Complex expressions with multiple operators and parentheses
            ("(A = 0x42 AND X < 0x10) OR (Y > 0x20 AND cycle_count < 0x100)", Box::new(|expr: &BooleanExpression| {
                if let BooleanExpression::Or(left, right) = expr {
                    matches!(**left, BooleanExpression::And(_, _)) &&
                    matches!(**right, BooleanExpression::And(_, _))
                } else {
                    false
                }
            })),
            
            // NOT operator tests
            ("NOT A = 0x42", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(inner) if matches!(**inner, BooleanExpression::Equal(
                    Source::Register(RegisterSource::Accumulator),
                    Source::Value(0x42)
                )))
            })),

            // NOT with parentheses
            ("NOT (A = 0x42)", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(inner) if matches!(**inner, BooleanExpression::Equal(
                    Source::Register(RegisterSource::Accumulator),
                    Source::Value(0x42)
                )))
            })),

            // NOT with AND
            ("NOT (A = 0x42 AND X = 0x10)", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(inner) if matches!(**inner, BooleanExpression::And(_, _)))
            })),

            // NOT with OR
            ("NOT (A = 0x42 OR X = 0x10)", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(inner) if matches!(**inner, BooleanExpression::Or(_, _)))
            })),

            // Multiple NOTs (double negation)
            ("NOT NOT A = 0x42", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(outer) if matches!(**outer, BooleanExpression::Not(_)))
            })),

            // NOT with complex expressions
            ("NOT (A = 0x42 AND X < 0x10) OR Y > 0x20", Box::new(|expr: &BooleanExpression| {
                if let BooleanExpression::Or(left, right) = expr {
                    let left_matches = if let BooleanExpression::Not(inner) = &**left {
                        if let BooleanExpression::And(left_and, right_and) = &**inner {
                            matches!(&**left_and, BooleanExpression::Equal(
                                Source::Register(RegisterSource::Accumulator),
                                Source::Value(0x42)
                            )) && matches!(&**right_and, BooleanExpression::StrictlyLesser(
                                Source::Register(RegisterSource::RegisterX),
                                Source::Value(0x10)
                            ))
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    let right_matches = matches!(&**right, BooleanExpression::StrictlyGreater(
                        Source::Register(RegisterSource::RegisterY),
                        Source::Value(0x20)
                    ));
                    left_matches && right_matches
                } else {
                    false
                }
            })),

            // NOT with boolean literals
            ("NOT true", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(inner) if matches!(**inner, BooleanExpression::Value(true)))
            })),

            ("NOT false", Box::new(|expr: &BooleanExpression| {
                matches!(expr, BooleanExpression::Not(inner) if matches!(**inner, BooleanExpression::Value(false)))
            })),
        ];

        let context = create_test_context();
        
        for (input, matcher) in test_cases {
            let node = PestParser::parse(Rule::boolean_condition, input)
                .unwrap_or_else(|e| panic!("Failed to parse '{}': {}", input, e))
                .next()
                .expect("There should be one node in this input.");
            
            let output = context.parse_boolean_condition(node.into_inner()).unwrap();
            
            assert!(matcher(&output), 
                "Expression didn't match expected pattern for input: {}\nGot: {:?}", input, output);
        }
    }

    #[test]
    fn test_parse_hex_to_bytes() {
        let context = create_test_context();
        
        // Single byte values
        assert_eq!(vec![0x0F], context.parse_hex_to_bytes("F").unwrap());
        assert_eq!(vec![0xFF], context.parse_hex_to_bytes("FF").unwrap());
        
        // Multi-byte values
        assert_eq!(vec![0x0F, 0xFF], context.parse_hex_to_bytes("FFF").unwrap());
        assert_eq!(vec![0xFF, 0xFF], context.parse_hex_to_bytes("FFFF").unwrap());
    }

    #[test]
    fn test_parse_hex_values() {
        let context = create_test_context();
        
        // 8-bit values
        assert_eq!(0x0F, context.parse_hex("F").unwrap());
        assert_eq!(0xFF, context.parse_hex("FF").unwrap());
        
        // 16-bit values
        assert_eq!(0x0FFF, context.parse_hex("FFF").unwrap());
        assert_eq!(0xFFFF, context.parse_hex("FFFF").unwrap());
    }

    #[test]
    fn test_parse_byte_sequence() {
        let context = create_test_context();
        let result = context.parse_hex_sequence("F,FF,A").unwrap();
        assert_eq!(vec![0x0F, 0xFF, 0x0A], result);
    }

    #[test]
    fn test_hex_byte_ordering() {
        let context = create_test_context();
        
        // Test byte ordering with different positions of zero
        assert_eq!(0x8000, context.parse_hex("8000").unwrap(), "High byte should be preserved in first position");
        assert_eq!(0x0080, context.parse_hex("0080").unwrap(), "High byte should be preserved in second position");
        assert_eq!(0x1234, context.parse_hex("1234").unwrap(), "Bytes should be in correct order");
        assert_eq!(0x0012, context.parse_hex("0012").unwrap(), "Leading zeros should be preserved");
    }

    #[test]
    fn test_parse_memory_with_hex_address_formats() {
        let context = create_test_context();
        
        // Test full 4-digit address
        let input = "#0x1234";
        let node = PestParser::parse(Rule::hex_address, input)
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(0x1234, context.parse_memory(&node).unwrap());

        // Test 3-digit address
        let input = "#0x123";
        let node = PestParser::parse(Rule::hex_address, input)
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(0x0123, context.parse_memory(&node).unwrap());

        // Test 2-digit address
        let input = "#0x12";
        let node = PestParser::parse(Rule::hex_address, input)
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(0x0012, context.parse_memory(&node).unwrap());

        // Test single-digit address
        let input = "#0x1";
        let node = PestParser::parse(Rule::hex_address, input)
            .unwrap()
            .next()
            .unwrap();
        assert_eq!(0x0001, context.parse_memory(&node).unwrap());
    }

    #[test]
    fn test_parse_memory_with_hex_address_in_commands() {
        let context = create_test_context();

        // Test in memory write command
        let input = "memory write #0xF 0x(42,A)";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } 
                if address == 0x000F && bytes == vec![0x42, 0x0A])
        );

        // Test in assert command
        let input = "assert #0xF = 0xA  $$test short hex address and value$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs, &context).unwrap();
        assert!(
            matches!(command.condition, 
                BooleanExpression::Equal(
                    Source::Memory(addr),
                    Source::Value(val)
                ) if addr == 0x000F && val == 0x0A
            )
        );
    }

    // This is just to debug complex expressions and check they parse correctly
    #[test]
    fn test_show_parsed_command() {
        let mut symbols = setup_test_symbols();
        symbols.add_symbol(0x1000, "main".to_string());
        let context = ParserContext::new(Some(&symbols));
        
        // let input = "X >= 10 AND (CP >= $main AND CP <= 0x2100) AND cycle_count < 50";
        let input = "X >= 10 AND (CP >= $main AND CP <= 0x2100) AND cycle_count < 50";
        let node = PestParser::parse(Rule::boolean_condition, input)
            .unwrap()
            .next()
            .expect("There should be one node in this input.");
        
        let output = context.parse_boolean_condition(node.into_inner()).unwrap();
        
        // Helper function to recursively print the structure
        fn print_expr(expr: &BooleanExpression, indent: usize) -> String {
            let spaces = " ".repeat(indent);
            match expr {
                BooleanExpression::And(left, right) => {
                    format!("{}AND(\n{},\n{}\n{})", 
                        spaces,
                        print_expr(left, indent + 2),
                        print_expr(right, indent + 2),
                        spaces
                    )
                },
                BooleanExpression::Or(left, right) => {
                    format!("{}OR(\n{},\n{}\n{})", 
                        spaces,
                        print_expr(left, indent + 2),
                        print_expr(right, indent + 2),
                        spaces
                    )
                },
                BooleanExpression::Not(inner) => {
                    format!("{}NOT(\n{})", 
                        spaces,
                        print_expr(inner, indent + 2)
                    )
                },
                BooleanExpression::Equal(left, right) => {
                    format!("{}Equal({:?}, {:?})", spaces, left, right)
                },
                BooleanExpression::Different(left, right) => {
                    format!("{}Different({:?}, {:?})", spaces, left, right)
                },
                BooleanExpression::StrictlyGreater(left, right) => {
                    format!("{}StrictlyGreater({:?}, {:?})", spaces, left, right)
                },
                BooleanExpression::StrictlyLesser(left, right) => {
                    format!("{}StrictlyLesser({:?}, {:?})", spaces, left, right)
                },
                BooleanExpression::GreaterOrEqual(left, right) => {
                    format!("{}GreaterOrEqual({:?}, {:?})", spaces, left, right)
                },
                BooleanExpression::LesserOrEqual(left, right) => {
                    format!("{}LesserOrEqual({:?}, {:?})", spaces, left, right)
                },
                BooleanExpression::Value(val) => {
                    format!("{}Value({})", spaces, val)
                },
                BooleanExpression::MemorySequence(addr, bytes) => {
                    format!("{}MemorySequence({:?}, {:?})", spaces, addr, bytes)
                },
            }
        }

        println!("\nParsed condition structure:\n{}", print_expr(&output, 0));


    }

    #[test]
    fn test_parse_memory_with_memory_location() {
        let mut symbols = test_utils::setup_test_symbols();
        symbols.add_symbol(0x1000, "somewhere".to_string());
        let context = ParserContext::new(Some(&symbols));
        let addr = context.parse_memory(&PestParser::parse(Rule::memory_location, "$somewhere")
            .unwrap()
            .next()
            .unwrap())
            .unwrap();
        assert_eq!(addr, 0x1000);
    }
}