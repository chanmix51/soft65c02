use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

/// 65C02 code tester
/// This program allows step by step execution of 65C02 processor and assertions
/// on memory or registers.
/// It takes script as parameter (or standard input) to execute tests.
#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
pub struct CommandLineArguments {

    /// Test script file location ("-" to read from standard input)
    #[arg(short, long)]
    filepath: PathBuf,

    /// Do not stop execution when an assertion fails
    #[arg(short, long)]
    continue_on_failure: bool,

    /// Display not only assertion results but also setup commands output
    verbose: bool,
    
}
fn main() -> Result<()> {
    let parameters = CommandLineArguments::parse();

    Ok(())
}
