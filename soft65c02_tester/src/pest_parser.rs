use anyhow::anyhow;
use pest::{iterators::Pairs, Parser};
use pest_derive::Parser;

use crate::{commands::*, AppResult};

#[derive(Parser)]
#[grammar = "../rules.pest"]
struct PestParser;

struct HelpCommandParser;

impl HelpCommandParser {
    pub fn from_pairs(mut pairs: Pairs<'_, Rule>) -> AppResult<HelpCommand> {
        match pairs
            .next()
            .unwrap() // Must be a rule in this collection
            .as_rule()
        {
            Rule::help_global => Ok(HelpCommand::Global),
            Rule::help_run => Ok(HelpCommand::Run),
            Rule::help_registers => Ok(HelpCommand::Registers),
            _ => todo!(),
        }
    }
}

struct RunCommandParser;

impl RunCommandParser {
    pub fn from_pairs(pairs: Pairs<'_, Rule>) -> AppResult<RunCommand> {
        use crate::until_condition::BooleanExpression;

        let mut start_address = None;
        let mut stop_condition = BooleanExpression::Value(true);

        for pair in pairs {
            match pair.as_rule() {
                Rule::memory_address => {
                    start_address = Some(parse_memory(pair.as_str())?);
                }
                Rule::run_until_condition => todo!(),
                _ => todo!(),
            }
        }

        Ok(RunCommand {
            stop_condition,
            start_address,
        })
    }
}

struct CliCommandParser;

impl CliCommandParser {
    pub fn from_str(line: &str) -> AppResult<CliCommand> {
        let mut pairs = PestParser::parse(Rule::sentence, line)?
            .next()
            .unwrap() // There is only one instruction per input
            .into_inner();
        let command = if let Some(pair) = pairs.next() {
            match pair.as_rule() {
                Rule::help_instruction => {
                    CliCommand::Help(HelpCommandParser::from_pairs(pair.into_inner())?)
                }
                _ => todo!(),
            }
        } else {
            CliCommand::None
        };

        Ok(command)
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_global_help() {
        let command = CliCommandParser::from_str("help").unwrap();

        assert!(matches!(command, CliCommand::Help(HelpCommand::Global)));
    }

    #[test]
    fn test_help_run() {
        let command = CliCommandParser::from_str("help run").unwrap();

        assert!(matches!(command, CliCommand::Help(HelpCommand::Run)));
    }

    #[test]
    fn test_help_registers() {
        let command = CliCommandParser::from_str("help registers").unwrap();

        assert!(matches!(command, CliCommand::Help(HelpCommand::Registers)));
    }
}
