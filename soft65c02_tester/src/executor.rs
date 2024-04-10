use std::{
    io::{BufRead, Lines},
    sync::mpsc::Sender,
};

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

#[derive(Debug)]
struct CommandIterator<B>
where
    B: BufRead,
{
    iterator: Lines<B>,
}

impl<B> CommandIterator<B>
where
    B: BufRead,
{
    pub fn new(iterator: Lines<B>) -> Self {
        Self { iterator }
    }
}

impl<B> Iterator for CommandIterator<B>
where
    B: BufRead,
{
    type Item = AppResult<CliCommand>;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next().map(|result| {
            result
                .map_err(|e| anyhow!(e))
                .and_then(|line| CliCommandParser::from(&line))
        })
    }
}

#[derive(Debug)]
pub struct ExecutorConfiguration {
    stop_on_failure: bool,
}

impl Default for ExecutorConfiguration {
    fn default() -> Self {
        Self {
            stop_on_failure: true,
        }
    }
}

#[derive(Debug, Default)]
pub struct Executor {
    configuration: ExecutorConfiguration,
}

impl Executor {
    pub fn new(configuration: ExecutorConfiguration) -> Self {
        Self { configuration }
    }

    pub fn run<T: BufRead>(self, buffer: T, sender: Sender<OutputToken>) -> AppResult<()> {
        let mut round = ExecutionRound::default();

        for result in CommandIterator::new(buffer.lines()) {
            let command = match result {
                Err(e) if self.configuration.stop_on_failure => return Err(anyhow!(e)),
                Err(_) => continue,
                Ok(c) => c,
            };

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
    fn test_halt_on_error() {
        let configuration = ExecutorConfiguration::default();
        let executor = Executor::new(configuration);
        let buffer = "marker $$first thing$$\nazerty".as_bytes();
        let (sender, _receiver) = channel::<OutputToken>();

        let error = executor.run(buffer, sender).unwrap_err();

        assert!(error.to_string().contains("azerty"));
    }

    #[test]
    fn test_on_error_continue() {
        let configuration = ExecutorConfiguration {
            stop_on_failure: false,
        };
        let executor = Executor::new(configuration);
        let buffer =
            "marker $$first thing$$\nazerty\nassert A=0x00 $$accumulator is zero$$".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap();

        assert_eq!(2, receiver.iter().count());
    }

    #[test]
    fn test_execution_ok_without_initial_marker() {
        let lines = &[
            "memory write #0x0800 0x(a9,c0)", // LDA $c0
            "run #0x0800",
            "assert A=0xc0 $$accumulator is loaded$$",
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());

        executor.run(lines.as_bytes(), sender).unwrap();

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
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());
        executor.run(lines.as_bytes(), sender).unwrap();

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
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());

        executor.run(lines.as_bytes(), sender).unwrap();

        assert_eq!(3, receiver.iter().count());
    }

    #[test]
    fn test_several_plans() {
        let lines = &[
            "memory write #0x0800 0x(a9,c0)", // LDA $c0
            "assert A=0xc0 $$accumulator is loaded$$",
            "marker $$second test plan$$",
            "assert A=0x00 $$accumulator is zero$$",
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());

        executor.run(lines.as_bytes(), sender).unwrap();

        let output = receiver.iter().nth(3).expect("there shall be a 4th output");

        assert!(
            matches!(output, OutputToken::Assertion { success, description } if success && description.contains("zero"))
        );
    }
}
