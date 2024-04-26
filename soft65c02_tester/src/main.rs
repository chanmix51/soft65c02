use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    sync::mpsc::channel,
};

use anyhow::{anyhow, Result};
use clap::Parser;
use soft65c02_tester::{
    AppResult, CliCommand, CliDisplayer, CommandIterator, Displayer, Executor,
    ExecutorConfiguration, OutputToken,
};

/// 65C02 code tester
///
/// This program allows step by step execution of 65C02 processor and performs
/// assertions on memory or registers.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CommandLineArguments {
    /// Test script file location , "-" to read from standard input (default).
    #[arg(short, long, default_value = "-")]
    input_filepath: PathBuf,

    /// Test output filepath, "-" to write to standard output (default).
    #[arg(short, long, default_value = "-")]
    output_filepath: PathBuf,

    /// Do not stop execution of a plan when an assertion fails.
    #[arg(short, long)]
    continue_on_failure: bool,

    /// Display not only assertion results but also setup commands output.
    #[arg(short, long)]
    verbose: bool,

    /// Just parse the file without executing the tests.
    #[arg(short, long, default_value = "false")]
    parse: bool,
}

impl CommandLineArguments {
    pub fn get_input_file_path(&self) -> AppResult<PathBuf> {
        if self.read_from_standard_input() {
            Err(anyhow!("Reading from standard input."))
        } else if self.input_filepath.exists() {
            Ok(self.input_filepath.clone())
        } else {
            Err(anyhow!(
                "File not found: '{:?}'.",
                self.input_filepath.display()
            ))
        }
    }

    pub fn read_from_standard_input(&self) -> bool {
        self.input_filepath == PathBuf::from("-")
    }

    pub fn write_to_standard_output(&self) -> bool {
        self.output_filepath == PathBuf::from("-")
    }
}
fn main() -> Result<()> {
    let parameters = CommandLineArguments::parse();

    let input_buffer: Box<dyn BufRead> = if parameters.read_from_standard_input() {
        Box::new(std::io::stdin().lock())
    } else {
        Box::new(BufReader::new(File::open(
            parameters.get_input_file_path()?,
        )?))
    };
    if parameters.parse {
        let result: AppResult<Vec<CliCommand>> = CommandIterator::new(input_buffer.lines())
            .map(|result| result.map_err(anyhow::Error::from))
            .collect();

        return result.map(|_| ());
    }
    let output_buffer: Box<dyn Write + Sync + Send> = if parameters.write_to_standard_output() {
        Box::new(std::io::stdout())
    } else {
        Box::new(std::fs::File::create(parameters.output_filepath)?)
    };
    let (sender, receiver) = channel::<OutputToken>();
    let mut displayer = CliDisplayer::new(output_buffer, parameters.verbose);
    let handler = std::thread::spawn(move || displayer.display(receiver));
    let executor = Executor::new(ExecutorConfiguration {
        stop_on_failed_assertion: !parameters.continue_on_failure,
        ..Default::default()
    });
    let result = executor.run(input_buffer, sender);
    handler.join().map_err(|e| anyhow!("Join error: {e:?}"))??;

    result
}
