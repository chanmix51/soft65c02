use std::path::PathBuf;

use anyhow::anyhow;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::{
    commands::*,
    until_condition::{Assignment, BooleanExpression, RegisterSource, Source},
    atari_binary::AtariBinary,
    apple_single::AppleSingle,
    symbols::SymbolTable,
    AppResult,
};

#[derive(Parser)]
#[grammar = "../rules.pest"]
struct PestParser;

pub struct ParserContext<'a> {
    symbols: Option<&'a SymbolTable>,
}

impl<'a> ParserContext<'a> {
    pub fn new(symbols: Option<&'a SymbolTable>) -> Self {
        Self { symbols }
    }

    fn parse_hex(&self, hex_str: &str) -> AppResult<usize> {
        if hex_str.is_empty() {
            return Err(anyhow!("Empty string is not a valid Hexadecimal."));
        }
        let bytes = hex::decode(hex_str)?;
        let mut addr: usize = 0;
        for byte in bytes.iter() {
            addr = addr << 8 | (*byte as usize);
        }
        Ok(addr)
    }

    pub fn parse_memory(&self, pair: &Pair<Rule>) -> AppResult<usize> {
        match pair.as_rule() {
            Rule::memory_address => {
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

    pub fn parse_boolean_condition(&self, mut nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
        let node = nodes.next().unwrap();
        let expression = match node.as_rule() {
            Rule::boolean => BooleanExpression::Value(node.as_str() == "true"),
            Rule::comparison => self.parse_comparison(node.into_inner())?,
            Rule::memory_sequence => {
                let mut seq_nodes = node.into_inner();
                let addr_node = seq_nodes.next().unwrap();
                let addr = self.parse_source_memory(&addr_node)?;
                let bytes_list_node = seq_nodes.next().unwrap();
                let bytes_node = bytes_list_node.into_inner().next().unwrap();
                let bytes = self.parse_bytes(bytes_node.as_str())?;
                BooleanExpression::MemorySequence(addr, bytes)
            },
            smt => panic!("unknown node type '{smt:?}'. Is the Pest grammar up to date?"),
        };

        Ok(expression)
    }

    fn parse_comparison(&self, mut nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
        let node = nodes.next().unwrap();
        let lh = match node.as_rule() {
            Rule::register8 | Rule::register16 | Rule::register_cycle => self.parse_source_register(&node),
            Rule::memory_address => self.parse_source_memory(&node)?,
            Rule::value8 | Rule::value16 => self.parse_source_value(&node)?,
            v => panic!("unexpected node '{:?}' here.", v),
        };
        let middle_node = nodes.next().unwrap();
        let node = nodes.next().unwrap();
        
        let rh = match node.as_rule() {
            Rule::register8 | Rule::register16 | Rule::register_cycle => self.parse_source_register(&node),
            Rule::memory_address => self.parse_source_memory(&node)?,
            Rule::value8 | Rule::value16 => self.parse_source_value(&node)?,
            v => panic!("unexpected node '{:?}' here.", v),
        };

        let expression = match middle_node.as_str() {
            "=" => BooleanExpression::Equal(lh, rh),
            ">=" => BooleanExpression::GreaterOrEqual(lh, rh),
            ">" => BooleanExpression::StrictlyGreater(lh, rh),
            "<=" => BooleanExpression::LesserOrEqual(lh, rh),
            "<" => BooleanExpression::StrictlyLesser(lh, rh),
            "!=" => BooleanExpression::Different(lh, rh),
            v => panic!("unknown operator {:?}", v),
        };

        Ok(expression)
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

    fn parse_source_memory(&self, node: &Pair<Rule>) -> AppResult<Source> {
        Ok(Source::Memory(self.parse_memory(node)?))
    }

    fn parse_source_value(&self, node: &Pair<Rule>) -> AppResult<Source> {
        let value_str = &node.as_str()[2..]; // Skip the "0x" prefix
        let value = self.parse_hex(value_str)?;
        
        // Validate the value size matches the rule type
        match node.as_rule() {
            Rule::value8 => {
                if value > 0xFF {
                    return Err(anyhow!("Value 0x{:X} is too large for 8-bit value", value));
                }
            }
            Rule::value16 => {
                if value > 0xFFFF {
                    return Err(anyhow!("Value 0x{:X} is too large for 16-bit value", value));
                }
            }
            _ => panic!("Unexpected rule in parse_source_value: {:?}", node.as_rule()),
        }
        
        Ok(Source::Value(value))
    }

    fn parse_bytes(&self, bytes: &str) -> AppResult<Vec<u8>> {
        bytes
            .split(',')
            .map(|x| hex::decode(x.trim()).map(|v| v[0]).map_err(|e| anyhow!(e)))
            .collect()
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
                        c => panic!("unknown escape sequence '\\{}'", c),
                    }
                }
                c => bytes.push(c as u8),
            }
        }
        bytes
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
            Rule::symbol_load => self.handle_symbol_load(pair.into_inner())?,
            Rule::symbol_add => self.handle_symbol_add(pair.into_inner())?,
            _ => {
                panic!("Unexpected pair '{pair:?}'. memory_{{load,flush,write}} expected.");
            }
        };

        Ok(command)
    }

    fn handle_memory_write(&self, mut pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let addr_pair = pairs
            .next()
            .expect("there shall be a memory address argument to memory write");
        let address = self.context.parse_memory(&addr_pair)?;
        
        let bytes_node = pairs
            .next()
            .expect("There shall be some bytes to write to memory");
        
        let bytes = match bytes_node.as_rule() {
            Rule::bytes => self.context.parse_bytes(bytes_node.as_str())?,
            Rule::string_literal => {
                // Remove the quotes
                let str_content = &bytes_node.as_str()[1..bytes_node.as_str().len()-1];
                self.context.parse_string_literal(str_content)
            }
            _ => panic!("Expected bytes or string_literal in memory write")
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

    fn handle_address_load(&self, address_pair: Pair<'_, Rule>, filename_pair: Option<Pair<'_, Rule>>) -> AppResult<MemoryCommand> {
        let address = self.context.parse_memory(&address_pair)?;
        let filename = filename_pair
            .expect("there shall be a filename argument to memory load")
            .as_str();
        let filepath = PathBuf::from(&filename[1..filename.len() - 1]);

        Ok(MemoryCommand::Load { address, filepath })
    }

    fn handle_target_load(&self, target_pair: Pair<'_, Rule>, filename_pair: Option<Pair<'_, Rule>>) -> AppResult<MemoryCommand> {
        let target = target_pair.as_str();
        let filename = filename_pair
            .expect("there shall be a filename argument to memory load")
            .as_str();
        let filepath = PathBuf::from(&filename[1..filename.len() - 1]);

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
        let filepath = PathBuf::from(&filename[1..filename.len() - 1]);
        
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
                stmt => panic!("unknown node type {stmt:?}. Is the Pest grammar up to date?"),
            }
        }

        Ok(RunCommand {
            stop_condition,
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
                    _ => self.context.parse_boolean_condition(Pairs::single(first))?,
                }
            }
            _ => panic!("Expected boolean_condition, got {:?}", boolean_condition.as_rule()),
        };

        let comment = pairs.next().unwrap().as_str().to_string();
        let command = AssertCommand { comment, condition };

        Ok(command)
    }

    fn parse_memory_sequence(&self, node: Pair<Rule>) -> AppResult<BooleanExpression> {
        let mut seq_nodes = node.into_inner();
        let addr_node = seq_nodes.next().expect("memory_sequence should have a memory_address node");
        let addr = self.context.parse_source_memory(&addr_node)?;
        
        let sequence_node = seq_nodes.next().expect("memory_sequence should have a bytes_list or string_literal node");
        let bytes = match sequence_node.as_rule() {
            Rule::bytes_list => {
                let bytes_node = sequence_node.into_inner().next().expect("bytes_list should contain a bytes node");
                self.context.parse_bytes(bytes_node.as_str())?
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
            _ => {
                panic!(
                    "'{}' was not expected here: 'register|memory|run|assert|reset|symbols instruction'.",
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
}
