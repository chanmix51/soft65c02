use anyhow::anyhow;
use soft65c02_lib::{Memory, Registers};

use crate::{until_condition::BooleanExpression, AppResult};

pub trait Command {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<String>;
}

#[derive(Debug)]
pub enum CliCommand {
    Run(RunCommand),
    Assert(AssertCommand),
    None,
}

impl Command for CliCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<String> {
        match self {
            Self::Run(command) => command.execute(registers, memory),
            Self::Assert(command) => command.execute(registers, memory),
            Self::None => Ok(String::new()),
        }
    }
}

#[derive(Debug)]
pub struct AssertCommand {
    pub condition: BooleanExpression,
    pub comment: String,
}

impl Command for AssertCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<String> {
        if self.condition.solve(registers, memory) {
            Ok(self.comment.clone())
        } else {
            Err(anyhow!(self.comment.clone()))
        }
    }
}

#[derive(Debug)]
pub struct RunCommand {
    pub stop_condition: BooleanExpression,
    pub start_address: Option<usize>,
}

impl Command for RunCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<String> {
        todo!()
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

        match command.execute(&mut registers, &mut memory) {
            Ok(s) => assert_eq!("nice comment", s),
            Err(_) => panic!("This condition must be valid."),
        };
    }

    #[test]
    fn test_assert_command_fails() {
        let command = AssertCommand {
            condition: BooleanExpression::Value(false),
            comment: "nice comment".to_string(),
        };
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();

        command
            .execute(&mut registers, &mut memory)
            .expect_err("This condition must fail.");
    }
}

#[cfg(test)]
mod run_command_tests {
    use super::*;
}
