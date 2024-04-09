use std::sync::mpsc::Sender;

use anyhow::anyhow;
use soft65c02_lib::{Memory, Registers};

use crate::{AppResult, CliCommand, CliCommandParser, Command, OutputToken};

#[derive(Debug)]
struct ExecutionRound {
    registers: Registers,
    memory: Memory,
}

impl Default for ExecutionRound {
    fn default() -> Self {
        let registers = Registers::new_initialized(0);
        let memory = Memory::new_with_ram();

        Self { registers, memory }
    }
}

impl ExecutionRound {
    fn get_mut(&mut self) -> (&mut Registers, &mut Memory) {
        (&mut self.registers, &mut self.memory)
    }
}

#[derive(Debug, Default)]
pub struct Executor {
    commands: Vec<CliCommand>,
}

impl Executor {
    /// Constructor
    pub fn new(lines: &[&str]) -> AppResult<Self> {
        let commands: Vec<CliCommand> = lines
            .iter()
            .map(|line| CliCommandParser::from(line).map_err(|e| anyhow!(e)))
            .collect::<AppResult<Vec<CliCommand>>>()?;

        let myself = Self { commands };

        Ok(myself)
    }

    pub fn run(self, sender: Sender<OutputToken>) -> AppResult<()> {
        let mut round = ExecutionRound::default();

        for command in self.commands {
            if matches!(command, CliCommand::None) {
                continue;
            }
            let (registers, memory) = round.get_mut();
            let token = command.execute(registers, memory)?;

            if matches!(token, OutputToken::Marker { description: _ }) {
                round = ExecutionRound::default();
            }

            sender.send(token)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;

    use super::*;

    #[test]
    fn test_constructor_err() {
        let lines = &["marker $$first thing$$", "azerty"];
        let message = Executor::new(lines).unwrap_err().to_string();

        assert!(message.contains("azerty"));
    }

    #[test]
    fn test_constructor_ok() {
        let lines = &["marker $$first thing$$", "registers flush"];

        let _executor = Executor::new(lines).unwrap();
    }

    #[test]
    fn test_execution_ok_without_initial_marker() {
        let lines = &[
            "memory write #0x0800 0x(a9,c0)", // LDA $c0
            "run #0x0800",
            "assert A=0xc0 $$accumulator is loaded$$",
        ];
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(lines).unwrap();
        executor.run(sender).unwrap();

        let output = receiver.recv().unwrap();
        assert!(matches!(output, OutputToken::Setup(_)));

        let output = receiver.recv().unwrap();
        assert!(matches!(output, OutputToken::Run { loglines } if loglines.len() == 1));

        let output = receiver.recv().unwrap();
        assert!(
            matches!(output, OutputToken::Assertion { success, description } if success && description == *"accumulator is loaded")
        );

        assert!(receiver.recv().is_err());
    }

    #[test]
    fn test_execution_ok_with_initial_marker() {
        let lines = &[
            "marker $$load accumulator$$",
            "memory write #0x0800 0x(a9,c0)", // LDA $c0
            "run #0x0800",
            "assert A=0xc0 $$accumulator is loaded$$",
        ];
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(lines).unwrap();
        executor.run(sender).unwrap();

        let output = receiver.recv().unwrap();
        assert!(
            matches!(output, OutputToken::Marker { description } if description == *"load accumulator")
        );

        let output = receiver.recv().unwrap();
        assert!(matches!(output, OutputToken::Setup(_)));

        let output = receiver.recv().unwrap();
        assert!(matches!(output, OutputToken::Run { loglines } if loglines.len() == 1));

        let output = receiver.recv().unwrap();
        assert!(
            matches!(output, OutputToken::Assertion { success, description } if success && description == *"accumulator is loaded")
        );

        assert!(receiver.recv().is_err());
    }

    #[test]
    fn test_with_blank_lines() {
        let lines = &[
            "memory write #0x0800 0x(a9,c0)", // LDA $c0
            "",
            "run #0x0800",
            "   ",
            "assert A=0xc0 $$accumulator is loaded$$",
        ];
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(lines).unwrap();
        executor.run(sender).unwrap();

        assert_eq!(3, receiver.iter().count());
    }
}
