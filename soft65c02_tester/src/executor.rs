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
pub struct CommandIterator<B>
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

/// Configuration of the executor.
#[derive(Debug)]
pub struct ExecutorConfiguration {
    /// If true, the executor stops when a command cannot be parsed.
    pub stop_on_parse_error: bool,

    /// If true, the executor stops when an assertion fails.
    pub stop_on_failed_assertion: bool,
}

impl Default for ExecutorConfiguration {
    fn default() -> Self {
        Self {
            stop_on_parse_error: true,
            stop_on_failed_assertion: true,
        }
    }
}

/// The executor is responsible of running a test file. It sets up memory and
/// registers and maintain them during the execution of the plan. It ensures
/// that the process stops if the Command Pointer register is unchanged after a
/// command execution (if the configuration allows it) or when an error occures.
/// All outputs are sent to a channel receiver.
#[derive(Debug, Default)]
pub struct Executor {
    configuration: ExecutorConfiguration,
}

impl Executor {
    pub fn new(configuration: ExecutorConfiguration) -> Self {
        Self { configuration }
    }

    /// Execute the commands from the buffer and send the outputs to the sender.
    /// The execution stops if an error occurs if the configuration requires it.
    /// The execution stops if an assertion fails the configuration requires it.
    /// The execution stops if the buffer is exhausted.
    pub fn run<T: BufRead>(self, buffer: T, sender: Sender<OutputToken>) -> AppResult<()> {
        let mut round = ExecutionRound::default();

        for result in CommandIterator::new(buffer.lines()) {
            let command = match result {
                Err(e) if self.configuration.stop_on_parse_error => return Err(anyhow!(e)),
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
            } else if matches!(token, OutputToken::Assertion { ref failure, description: _ } if self.configuration.stop_on_failed_assertion && failure.is_some())
            {
                sender.send(token)?;

                return Err(anyhow!("Assertion failed"));
            }

            sender.send(token)?;
        }

        // buffer is exhausted
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
            stop_on_parse_error: false,
            ..ExecutorConfiguration::default()
        };
        let executor = Executor::new(configuration);
        let buffer =
            "marker $$first thing$$\nazerty\nassert A=0x00 $$accumulator is zero$$".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap();

        assert_eq!(2, receiver.iter().count());
    }

    #[test]
    fn test_halt_on_assertion_failed() {
        let configuration = ExecutorConfiguration::default();
        let executor = Executor::new(configuration);
        let buffer = "assert A=0x01 $$first test$$\nassert X=0x00 $$second test$$\n".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap_err();

        let output = receiver.recv().unwrap();
        assert!(
            matches!(output, OutputToken::Assertion { failure, description } if failure.is_some() && description == *"first test")
        );

        // second test is not executed
        receiver.recv().unwrap_err();
    }

    #[test]
    fn test_continue_when_assertion_succeed() {
        let configuration = ExecutorConfiguration::default();
        let executor = Executor::new(configuration);
        let buffer = "assert A=0x00 $$first test$$\nassert X=0x00 $$second test$$\n".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap();

        assert_eq!(2, receiver.into_iter().count());
    }

    #[test]
    fn test_continue_when_assertion_failed() {
        let configuration = ExecutorConfiguration {
            stop_on_failed_assertion: false,
            ..Default::default()
        };
        let executor = Executor::new(configuration);
        let buffer = "assert A=0x01 $$first test$$\nassert X=0x00 $$second test$$\n".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap();

        assert_eq!(2, receiver.into_iter().count());
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
            matches!(output, OutputToken::Assertion { failure, description } if failure.is_none() && description == *"accumulator is loaded")
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
            matches!(output, OutputToken::Assertion { failure, description } if failure.is_none() && description == *"accumulator is loaded")
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
            "registers set A=0xc0",
            "assert A=0xc0 $$accumulator is loaded$$",
            "marker $$second test plan$$",
            "assert A=0x00 $$accumulator is zero$$",
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());

        executor.run(lines.as_bytes(), sender).unwrap();

        let output = receiver
            .iter()
            .nth(3)
            .expect("there shall be a 4th output token");

        assert!(
            matches!(output, OutputToken::Assertion { failure, description } if failure.is_none() && description.contains("zero"))
        );
    }
}
