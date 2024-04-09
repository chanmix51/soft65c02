use anyhow::anyhow;
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use pest_derive::Parser;

use crate::{
    commands::*,
    until_condition::{BooleanExpression, Source},
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
            _ => {
                panic!("Unexpected pair '{pair:?}'. memory_{{load,flush,write}} expected.");
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
}
pub struct RegisterCommandParser;

impl RegisterCommandParser {
    pub fn from_pairs(pairs: Pairs<'_, Rule>) -> AppResult<RegisterCommand> {
        let mut pairs = pairs;
        let pair = pairs.next().unwrap();

        let command = match pair.as_rule() {
            Rule::registers_flush => RegisterCommand::Flush,
            _ => {
                panic!("Unexpected rule '{}', register rule was expected.", pair);
            }
        };

        Ok(command)
    }
}

#[cfg(test)]
mod register_parser_tests {
    use super::*;

    #[test]
    fn test_register_flush() {
        let input = "registers flush";
        let pairs = PestParser::parse(Rule::registers_instruction, input)
            .unwrap()
            .next()
            .unwrap()
            .into_inner();
        let command = RegisterCommandParser::from_pairs(pairs).unwrap();

        assert!(matches!(command, RegisterCommand::Flush));
    }
}
pub struct RunCommandParser;

impl RunCommandParser {
    pub fn from_pairs(pairs: Pairs<'_, Rule>) -> AppResult<RunCommand> {
        let mut start_address = None;
        let mut stop_condition = BooleanExpression::Value(true);

        for pair in pairs {
            match pair.as_rule() {
                Rule::memory_address => {
                    start_address = Some(parse_memory(&pair.as_str()[3..])?);
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
        assert!(matches!(command.start_address, Some(addr) if addr == 0x1234));
    }

    #[test]
    fn run_with_stop_condition() {
        let input = "run until A > 0x12";
        let mut parser = PestParser::parse(Rule::run_instruction, input).unwrap();
        let command = RunCommandParser::from_pairs(parser.next().unwrap().into_inner()).unwrap();

        if let BooleanExpression::StrictlyGreater(lt, rt) = command.stop_condition {
            assert!(matches!(lt, Source::Accumulator));
            assert!(matches!(rt, Source::Value(data) if data == 0x12));
        } else {
            panic!(
                "Expected StrictlyGreater boolean expression, got '{:?}'.",
                command.stop_condition
            );
        }
        assert!(command.start_address.is_none());
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
        Rule::operation => parse_operation(node.into_inner())?,
        smt => panic!("unknown node type '{smt:?}'. Is the Pest grammar up to date?"),
    };

    Ok(expression)
}

fn parse_operation(mut nodes: Pairs<Rule>) -> AppResult<BooleanExpression> {
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
        "A" => Source::Accumulator,
        "X" => Source::RegisterX,
        "Y" => Source::RegisterY,
        "S" => Source::RegisterS,
        "SP" => Source::RegisterSP,
        "CP" => Source::RegisterCP,
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
            BooleanExpression::Different(Source::Accumulator, Source::Value(0xff))
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
