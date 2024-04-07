use anyhow::anyhow;
use soft65c02_lib::{execute_step, AddressableIO, Memory, Registers};

use crate::{until_condition::BooleanExpression, AppResult};

pub trait Command {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<Vec<String>>;
}

#[derive(Debug)]
pub enum CliCommand {
    Assert(AssertCommand),
    Marker(String),
    Memory(MemoryCommand),
    None,
    Registers(RegisterCommand),
    Run(RunCommand),
}

impl Command for CliCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<Vec<String>> {
        match self {
            Self::Assert(command) => command.execute(registers, memory),
            Self::Marker(comment) => Ok(vec![comment.to_owned()]),
            Self::Memory(command) => command.execute(registers, memory),
            Self::None => Ok(Vec::new()),
            Self::Registers(command) => command.execute(registers, memory),
            Self::Run(command) => command.execute(registers, memory),
        }
    }
}

#[derive(Debug)]
pub struct AssertCommand {
    pub condition: BooleanExpression,
    pub comment: String,
}

impl Command for AssertCommand {
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<Vec<String>> {
        if self.condition.solve(registers, memory) {
            Ok(vec![self.comment.clone()])
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
    fn execute(&self, registers: &mut Registers, memory: &mut Memory) -> AppResult<Vec<String>> {
        if let Some(addr) = self.start_address {
            registers.command_pointer = addr;
        }

        let mut loglines: Vec<String> = Vec::new();
        let mut cp = registers.command_pointer;

        loop {
            loglines.push(
                execute_step(registers, memory)
                    .map(|l| l.to_string())
                    .map_err(|e| anyhow!(e))?,
            );

            if registers.command_pointer == cp || self.stop_condition.solve(registers, memory) {
                break;
            }
            cp = registers.command_pointer;
        }

        Ok(loglines)
    }
}

#[derive(Debug)]
pub enum RegisterCommand {
    Flush,
}

impl Command for RegisterCommand {
    fn execute(&self, registers: &mut Registers, _memory: &mut Memory) -> AppResult<Vec<String>> {
        registers.initialize(0x0000);

        Ok(Vec::new())
    }
}

#[derive(Debug)]
pub enum MemoryCommand {
    Flush,
    Load(String),
    Write { address: u16, bytes: Vec<u8> },
}

impl Command for MemoryCommand {
    fn execute(&self, _registers: &mut Registers, memory: &mut Memory) -> AppResult<Vec<String>> {
        let output = match self {
            Self::Flush => {
                *memory = Memory::new_with_ram();
                Vec::new()
            }
            Self::Write { address, bytes } => match bytes.len() {
                0 => vec!["nothing was written".to_string()],
                1 => {
                    memory.write(*address as usize, bytes)?;
                    vec!["1 byte written".to_string()]
                }
                _ => {
                    memory.write(*address as usize, bytes)?;
                    vec![format!("{} bytes written", bytes.len())]
                }
            },
            _ => todo!(),
        };

        Ok(output)
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
            Ok(s) => assert_eq!("nice comment", s[0]),
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
    use soft65c02_lib::AddressableIO;

    use crate::until_condition::Source;

    use super::*;

    #[test]
    fn simple_run() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(true),
            start_address: None,
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa9, 0xc0]).unwrap(); // LDA #0xc0
        let loglines = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!(1, loglines.len());
    }

    #[test]
    fn run_from_addr() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(true),
            start_address: Some(0x1234),
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1234, &[0xa9, 0xc0]).unwrap(); // LDA #0xc0
        let loglines = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!(1, loglines.len());
    }

    #[test]
    fn run_with_condition() {
        let command = RunCommand {
            stop_condition: BooleanExpression::StrictlyGreater(Source::RegisterX, Source::Value(0)),
            start_address: Some(0x1234),
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        //memory.write(0x1234, &[0xa9, 0xc0, 0xaa]).unwrap(); // LDA #0xc0; TXA
        let loglines = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!(2, loglines.len());
    }

    #[test]
    fn run_stops_on_loop() {
        let command = RunCommand {
            stop_condition: BooleanExpression::Value(false),
            start_address: None,
        };
        let mut registers = Registers::new_initialized(0x1000);
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xd0, 0b11111110]).unwrap(); // BNE -1
        let loglines = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!(1, loglines.len());
    }
}

#[cfg(test)]
mod register_command_tests {
    use super::*;

    #[test]
    fn test_show() {
        let command = RegisterCommand::Flush;
        let mut registers = Registers::new_initialized(0xffff);
        let mut memory = Memory::new_with_ram();
        let output = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!(0, output.len());
        assert_eq!(0x0000, registers.command_pointer);
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
        let output = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!(vec![0x00, 0x00, 0x00], memory.read(0x000, 3).unwrap());
        assert_eq!(0, output.len());
    }

    #[test]
    fn test_write_command() {
        let command = MemoryCommand::Write {
            address: 0x1000,
            bytes: vec![0x01, 0x02, 0x03],
        };
        let mut registers = Registers::new_initialized(0x0000);
        let mut memory = Memory::new_with_ram();
        let outputs = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!("3 bytes written", (outputs[0]));
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
        let outputs = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!("nothing was written", (outputs[0]));
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
        let outputs = command.execute(&mut registers, &mut memory).unwrap();

        assert_eq!("1 byte written", (outputs[0]));
        assert_eq!(
            &[0x01, 0x00, 0x00],
            memory.read(0x1000, 3).unwrap().as_slice()
        );
    }
}

#[cfg(test)]
mod cli_command_tests {
    use crate::CliCommandParser;

    use super::*;

    #[test]
    fn test_assertion() {
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();

        let output = CliCommandParser::from("assert #0x0000 = 0x00 $$The first byte is zero$$")
            .unwrap()
            .execute(&mut registers, &mut memory)
            .unwrap();

        assert_eq!("The first byte is zero".to_string(), output[0]);
    }

    #[test]
    fn test_bad_assertion() {
        let mut registers = Registers::new(0x0000);
        let mut memory = Memory::new_with_ram();

        let output =
            CliCommandParser::from("assert #0x0000 = 0x01 $$The first byte is one, really?$$")
                .unwrap()
                .execute(&mut registers, &mut memory)
                .unwrap_err();

        assert_eq!(
            "The first byte is one, really?".to_string(),
            output.to_string()
        );
    }

    #[test]
    fn test_register_flush() {
        let mut registers = Registers::new(0x1234);
        let mut memory = Memory::new_with_ram();

        let output = CliCommandParser::from("registers flush")
            .unwrap()
            .execute(&mut registers, &mut memory)
            .unwrap();

        assert_eq!(0, output.len());
    }
}
