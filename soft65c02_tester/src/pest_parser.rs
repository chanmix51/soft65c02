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
    AppResult,
};

#[derive(Parser)]
#[grammar = "../rules.pest"]
struct PestParser;

pub struct MemoryCommandParser;

impl MemoryCommandParser {
    pub fn from_pairs(pairs: Pairs<'_, Rule>) -> AppResult<MemoryCommand> {
        let mut pairs = pairs;
        let pair = pairs.next().unwrap();

        let command = match pair.as_rule() {
            Rule::memory_flush => MemoryCommand::Flush,
            Rule::memory_write => {
                let mut pairs = pair.into_inner();
                let address = parse_memory(
                    &pairs
                        .next()
                        .expect("there shall be a memory address argument to memory write")
                        .as_str()[3..],
                )?;
                let bytes = parse_bytes(
                    pairs
                        .next()
                        .expect("There shall be some bytes to write to memory.")
                        .as_str(),
                )?;
                MemoryCommand::Write { address, bytes }
            }
            Rule::memory_load => {
                let mut pairs = pair.into_inner();
                let address = parse_memory(
                    &pairs
                        .next()
                        .expect("there shall be a memory address argument to memory load")
                        .as_str()[3..],
                )?;
                let filename = pairs
                    .next()
                    .expect("there shall be a filename argument to memory load")
                    .as_str();
                let filepath = PathBuf::from(&filename[1..filename.len() - 1]);

                MemoryCommand::Load { address, filepath }
            }
            Rule::memory_load_atari => {
                let mut pairs = pair.into_inner();
                let _address = parse_memory(
                    &pairs
                        .next()
                        .expect("there shall be a memory address argument to memory load_atari")
                        .as_str()[3..],
                )?;
                let filename = pairs
                    .next()
                    .expect("there shall be a filename argument to memory load_atari")
                    .as_str();
                let filepath = PathBuf::from(&filename[1..filename.len() - 1]);
                
                // Use AtariBinary::from_file helper
                let binary = AtariBinary::from_file(&filepath)?;

                // Convert to memory segments
                let segments = binary.into_memory_segments();
                MemoryCommand::LoadSegments { segments }
            }
            _ => {
                panic!("Unexpected pair '{pair:?}'. memory_{{load,flush,write,load_atari}} expected.");
            }
        };

        Ok(command)
    }
}

#[cfg(test)]
mod memory_command_parser_tests {
    use super::*;

    #[test]
    fn test_memory_flush() {
        let input = "memory flush";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs).unwrap();

        assert!(matches!(command, MemoryCommand::Flush));
    }

    #[test]
    fn test_memory_write() {
        let input = "memory write #0x1234 0x(01,02,03)";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs).unwrap();

        assert!(
            matches!(command, MemoryCommand::Write { address, bytes } if address == 0x1234 && bytes == vec![0x01, 0x02, 0x03])
        );
    }

    #[test]
    fn test_memory_load() {
        let input = "memory load #0x1000 \"script.txt\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = MemoryCommandParser::from_pairs(pairs).unwrap();

        assert!(
            matches!(command, MemoryCommand::Load { address, filepath } if address == 0x1000 && filepath == PathBuf::from("script.txt"))
        );
    }

    #[test]
    fn test_memory_load_atari_command_parsing() {
        let input = "memory load_atari #0x1000 \"test.com\"";
        let pairs = PestParser::parse(Rule::memory_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let pair = pairs.into_iter().next().unwrap();

        // Verify it's the correct rule type
        assert!(matches!(pair.as_rule(), Rule::memory_load_atari));

        // Verify the inner parts (address and filename)
        let mut inner = pair.into_inner();
        let addr = inner.next().unwrap();
        assert_eq!(addr.as_str(), "#0x1000");
        let filename = inner.next().unwrap();
        assert_eq!(filename.as_str(), "\"test.com\"");
    }
}
pub struct RegisterCommandParser;

impl RegisterCommandParser {
    pub fn from_pairs(pairs: Pairs<'_, Rule>) -> AppResult<RegisterCommand> {
        let mut pairs = pairs;
        let pair = pairs.next().unwrap();

        let command = match pair.as_rule() {
            Rule::registers_flush => RegisterCommand::Flush,
            Rule::registers_set => Self::parse_register_set(pair.into_inner())?,
            _ => {
                panic!("Unexpected rule '{}', register rule was expected.", pair);
            }
        };

        Ok(command)
    }

    fn parse_register_set(pairs: Pairs<'_, Rule>) -> AppResult<RegisterCommand> {
        let mut pairs = pairs;
        let assignment = pairs.next().unwrap();
        let mut assignment = assignment.into_inner();
        let destination_node = assignment
            .next()
            .ok_or_else(|| anyhow!("expected a destination for register assignment"))?;
        let destination = match destination_node.as_rule() {
            Rule::register8 => match destination_node.as_str() {
                "A" => RegisterSource::Accumulator,
                "X" => RegisterSource::RegisterX,
                "Y" => RegisterSource::RegisterY,
                "S" => RegisterSource::Status,
                "SP" => RegisterSource::StackPointer,
                "CP" => RegisterSource::CommandPointer,
                v => panic!("unknown destination 8 bits register type '{:?}'.", v),
            },
            Rule::register16 => match destination_node.as_str() {
                "CP" => RegisterSource::CommandPointer,
                v => panic!("unknown destination 16 bits register type '{:?}'.", v),
            },
            v => panic!("unexpected node '{:?}' here.", v),
        };
        let source_node = assignment.next().unwrap();
        let source = match source_node.as_rule() {
            Rule::register8 => match source_node.as_str() {
                "A" => Source::Register(RegisterSource::Accumulator),
                "X" => Source::Register(RegisterSource::RegisterX),
                "Y" => Source::Register(RegisterSource::RegisterY),
                "S" => Source::Register(RegisterSource::Status),
                "SP" => Source::Register(RegisterSource::StackPointer),
                "CP" => Source::Register(RegisterSource::CommandPointer),
                v => panic!("unknown source register type '{:?}'.", v),
            },
            Rule::value8 => parse_source_value(&source_node)?,
            Rule::value16 => parse_source_value(&source_node)?,
            v => panic!("unexpected node '{:?}' here.", v),
        };

        Ok(RegisterCommand::Set {
            assignment: Assignment {
                destination,
                source,
            },
        })
    }
}

#[cfg(test)]
mod register_parser_tests {
    use super::*;

    #[test]
    fn test_registers_flush() {
        let input = "registers flush";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs).unwrap();

        assert!(matches!(command, RegisterCommand::Flush));
    }

    #[test]
    fn test_registers_set_value8() {
        let input = "registers set A=0xc0";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs).unwrap();

        assert!(matches!(
        command,
        RegisterCommand::Set {assignment}
        if matches!(assignment.destination, RegisterSource::Accumulator)
            && matches!(assignment.source, Source::Value(d) if d == 0xc0)
        ));
    }

    #[test]
    fn test_registers_set_value16() {
        let input = "registers set CP=0xc0ff";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs).unwrap();
        println!("{command:?}");

        assert!(matches!(
        command,
        RegisterCommand::Set {assignment}
        if matches!(assignment.destination, RegisterSource::CommandPointer)
            && matches!(assignment.source, Source::Value(d) if d == 0xc0ff)
        ));
    }
}
pub struct RunCommandParser;

impl RunCommandParser {
    pub fn from_pairs(pairs: Pairs<'_, Rule>) -> AppResult<RunCommand> {
        let mut start_address = None;
        let mut stop_condition = BooleanExpression::Value(true);

        for pair in pairs {
            match pair.as_rule() {
                Rule::run_address => {
                    if pair.as_str() == "init" {
                        start_address = Some(RunAddress::InitVector);
                    } else {
                        start_address =
                            Some(RunAddress::Memory(parse_memory(&pair.as_str()[3..])?));
                    };
                }
                Rule::run_until_condition => {
                    stop_condition =
                        parse_boolean_condition(pair.into_inner().next().unwrap().into_inner())?;
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
    use crate::until_condition::RegisterSource;

    use super::*;

    #[test]
    fn simple_run() {
        let input = "run";
        let pairs = PestParser::parse(Rule::run_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RunCommandParser::from_pairs(pairs).unwrap();

        assert!(matches!(command.stop_condition, BooleanExpression::Value(v) if v));
        assert!(command.start_address.is_none());
    }

    #[test]
    fn run_with_start_address() {
        let input = "run #0x1234";
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner()).unwrap();

        assert!(matches!(command.stop_condition, BooleanExpression::Value(v) if v));
        assert!(matches!(command.start_address, Some(RunAddress::Memory(addr)) if addr == 0x1234));
    }

    #[test]
    fn run_with_stop_condition() {
        let input = "run until A > 0x12";
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner()).unwrap();

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
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner()).unwrap();

        assert!(matches!(
            command.start_address,
            Some(RunAddress::InitVector)
        ));
    }
}

pub struct AssertCommandParser;

impl AssertCommandParser {
    pub fn from_pairs(mut pairs: Pairs<'_, Rule>) -> AppResult<AssertCommand> {
        // let mut pairs = pairs.next().unwrap().into_inner();
        let condition = parse_boolean_condition(pairs.next().unwrap().into_inner())?;
        let comment = pairs.next().unwrap().as_str().to_string();
        let command = AssertCommand { comment, condition };

        Ok(command)
    }
}

#[cfg(test)]
mod assert_parser_tests {
    use super::*;

    #[test]
    fn test_assert_parser() {
        let input = "assert A = 0x00 $$something$$";
        let pairs = PestParser::parse(Rule::assert_instruction, input)
            .unwrap()
            .next()
            .expect("one instruction per line")
            .into_inner();
        let command = AssertCommandParser::from_pairs(pairs).unwrap();

        assert!(
            matches!(command, AssertCommand { comment, condition } if comment.as_str() == "something" && matches!(condition, BooleanExpression::Equal(_, _)))
        )
    }
}

pub struct CliCommandParser;

impl CliCommandParser {
    pub fn from(line: &str) -> AppResult<CliCommand> {
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
                CliCommand::Run(RunCommandParser::from_pairs(pair.into_inner())?)
            }
            Rule::assert_instruction => {
                CliCommand::Assert(AssertCommandParser::from_pairs(pair.into_inner())?)
            }
            Rule::marker => {
                let marker = pair.into_inner().next().unwrap().as_str();
                CliCommand::Marker(marker.to_owned())
            }
            Rule::registers_instruction => {
                CliCommand::Registers(RegisterCommandParser::from_pairs(pair.into_inner())?)
            }
            Rule::memory_instruction => {
                CliCommand::Memory(MemoryCommandParser::from_pairs(pair.into_inner())?)
            }
            _ => {
                panic!(
                    "'{}' was not expected here: 'register|memory|run|assert|reset instruction'.",
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
}

fn parse_memory(addr: &str) -> AppResult<usize> {
    if addr.is_empty() {
        return Err(anyhow!("Empty string is not a valid Hexadecimal."));
    }

    let bytes = hex::decode(addr)?;
    let mut addr: usize = 0;

    for byte in bytes.iter() {
        addr = addr << 8 | (*byte as usize);
    }

    Ok(addr)
}

pub fn parse_boolean_condition(mut nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
    let node = nodes.next().unwrap();
    let expression = match node.as_rule() {
        Rule::boolean => BooleanExpression::Value(node.as_str() == "true"),
        Rule::comparison => parse_comparison(node.into_inner())?,
        smt => panic!("unknown node type '{smt:?}'. Is the Pest grammar up to date?"),
    };

    Ok(expression)
}

fn parse_comparison(mut nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
    let node = nodes.next().unwrap();
    let lh = match node.as_rule() {
        Rule::register8 | Rule::register16 => parse_source_register(&node),
        Rule::memory_address => parse_source_memory(&node)?,
        Rule::value8 | Rule::value16 => parse_source_value(&node)?,
        v => panic!("unexpected node '{:?}' here.", v),
    };
    let middle_node = nodes.next().unwrap();
    let node = nodes.next().unwrap();
    let rh = match node.as_rule() {
        Rule::register8 | Rule::register16 => parse_source_register(&node),
        Rule::memory_address => parse_source_memory(&node)?,
        Rule::value8 | Rule::value16 => parse_source_value(&node)?,
        v => panic!("unexpected node '{:?}' here.", v),
    };
    let expression = match middle_node.as_str() {
        "=" => BooleanExpression::Equal(lh, rh),
        ">=" => BooleanExpression::GreaterOrEqual(lh, rh),
        ">" => BooleanExpression::StrictlyGreater(lh, rh),
        "<=" => BooleanExpression::LesserOrEqual(lh, rh),
        "<" => BooleanExpression::StrictlyLesser(lh, rh),
        "!=" => BooleanExpression::Different(lh, rh),
        v => panic!("unknown 8 bits provider {:?}", v),
    };

    Ok(expression)
}

fn parse_source_register(node: &Pair<Rule>) -> Source {
    match node.as_str() {
        "A" => Source::Register(RegisterSource::Accumulator),
        "X" => Source::Register(RegisterSource::RegisterX),
        "Y" => Source::Register(RegisterSource::RegisterY),
        "S" => Source::Register(RegisterSource::Status),
        "SP" => Source::Register(RegisterSource::StackPointer),
        "CP" => Source::Register(RegisterSource::CommandPointer),
        v => panic!("unknown register type '{:?}'.", v),
    }
}

fn parse_source_memory(node: &Pair<Rule>) -> AppResult<Source> {
    let addr = parse_memory(&node.as_str()[3..])?;

    Ok(Source::Memory(addr))
}

fn parse_source_value(node: &Pair<Rule>) -> AppResult<Source> {
    let addr = parse_memory(&node.as_str()[2..])?;

    Ok(Source::Value(addr))
}

#[allow(dead_code)]
fn parse_bytes(bytes: &str) -> AppResult<Vec<u8>> {
    bytes
        .split(',')
        .map(|x| hex::decode(x.trim()).map(|v| v[0]).map_err(|e| anyhow!(e)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::until_condition::{BooleanExpression, Source};

    #[test]
    fn test_parse_boolean_condition() {
        let input = "A != 0xff";
        let node = PestParser::parse(Rule::boolean_condition, input)
            .unwrap()
            .next()
            .expect("There is one node in this input.");
        let output = parse_boolean_condition(node.into_inner()).unwrap();

        assert!(matches!(
            output,
            BooleanExpression::Different(
                Source::Register(RegisterSource::Accumulator),
                Source::Value(0xff)
            )
        ));
    }

    #[test]
    fn test_parse_memory_ok() {
        assert_eq!(0x02ff, parse_memory("02ff").unwrap());
        assert_eq!(0x0000, parse_memory("0000").unwrap());
        assert_eq!(0xea, parse_memory("ea").unwrap());
    }

    #[test]
    fn test_parse_memory_bad() {
        parse_memory("").expect_err("Empty string must yield an error.");
        parse_memory("   ").expect_err("Invisible string must yield an error.");
        parse_memory("xxx").expect_err("Non hexa must yield an error.");
    }
}
