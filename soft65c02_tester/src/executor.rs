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
    failed: bool,
}

impl Default for ExecutionRound {
    fn default() -> Self {
        let registers = Registers::new(0x0000);
        let memory = Memory::new_with_ram();
        let failed = false;

        Self {
            registers,
            memory,
            failed,
        }
    }
}

impl ExecutionRound {
    fn get_mut(&mut self) -> (&mut Registers, &mut Memory) {
        (&mut self.registers, &mut self.memory)
    }

    fn is_ok(&self) -> bool {
        !self.failed
    }

    fn set_failed(&mut self) {
        self.failed = true;
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
    pub ignore_parse_error: bool,

    /// If true, the executor stops when an assertion fails.
    pub stop_on_failed_assertion: bool,
}

impl Default for ExecutorConfiguration {
    fn default() -> Self {
        Self {
            ignore_parse_error: false,
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
    /// The execution stops if the buffer is exhausted. If an assertion fails
    /// and the configuration allows it, the execution stops until the next
    /// marker.
    pub fn run<T: BufRead>(self, buffer: T, sender: Sender<OutputToken>) -> AppResult<()> {
        let mut round = ExecutionRound::default();
        let mut failed: usize = 0;

        for result in CommandIterator::new(buffer.lines()) {
            let command = match result {
                Err(e) if !self.configuration.ignore_parse_error => return Err(anyhow!(e)),
                Err(_) => continue,
                Ok(c) => c,
            };

            if matches!(command, CliCommand::None) {
                continue;
            } else if matches!(command, CliCommand::Marker(_)) {
                round = ExecutionRound::default();
            } else if !round.is_ok() && self.configuration.stop_on_failed_assertion {
                continue;
            }
            let (registers, memory) = round.get_mut();
            let token = command.execute(registers, memory)?;

            if matches!(token, OutputToken::Assertion { ref failure, description: _ } if failure.is_some())
            {
                failed += 1;
                round.set_failed();
            }

            sender.send(token)?;
        }

        // buffer is exhausted
        if failed > 0 {
            Err(anyhow!("{failed} assertions failed!"))
        } else {
            Ok(())
        }
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
        let buffer = "marker $$first thing$$\nazerty\nassert true $$not executed$$".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        let error = executor.run(buffer, sender).unwrap_err();

        assert!(error.to_string().contains("azerty"));
        assert_eq!(1, receiver.iter().count());
    }

    #[test]
    fn test_on_error_continue() {
        let configuration = ExecutorConfiguration {
            ignore_parse_error: true,
            ..ExecutorConfiguration::default()
        };
        let executor = Executor::new(configuration);
        let buffer = "marker $$first thing$$\nazerty\nassert true $$shall pass$$".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap();

        assert_eq!(2, receiver.iter().count());
    }

    #[test]
    fn test_halt_on_assertion_failed() {
        let configuration = ExecutorConfiguration::default();
        let executor = Executor::new(configuration);
        let buffer = "assert false $$first test$$\nassert true $$second test$$\n".as_bytes();
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
        let buffer = "assert true $$first test$$\nassert true $$second test$$\n".as_bytes();
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
        let buffer = "assert false $$first test$$\nassert true $$second test$$\n".as_bytes();
        let (sender, receiver) = channel::<OutputToken>();

        executor.run(buffer, sender).unwrap_err();

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
            "assert A!=0xc0 $$accumulator is random$$",
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());

        let _ = executor.run(lines.as_bytes(), sender);

        let _output = receiver
            .iter()
            .nth(3)
            .expect("there shall be a 4th output token");
    }

    #[test]
    fn several_plans_with_one_failing() {
        let lines = &[
            "marker $$first plan$$",
            "assert false $$failing test$$",
            "assert true  $$must not be executed$$",
            "marker $$second plan$$",
            "assert true  $$must be executed$$",
            "assert false $$failing test$$",
            "marker $$third plan$$",
            "assert true $$must be executed$$",
        ]
        .join("\n");
        let (sender, receiver) = channel::<OutputToken>();
        let executor = Executor::new(ExecutorConfiguration::default());

        executor.run(lines.as_bytes(), sender).unwrap_err();
        let output = receiver
            .iter()
            .nth(2)
            .expect("there shall be a 3th output token");

        assert!(
            matches!(output, OutputToken::Marker { description } if description == *"second plan")
        );
        let output = receiver
            .iter()
            .nth(2)
            .expect("there shall be a 3th output token");

        assert!(
            matches!(output, OutputToken::Marker { description } if description == *"third plan")
        );
    }
}
