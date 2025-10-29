use std::path::PathBuf;
use clap::Parser;
use anyhow::Result;
use soft65c02_unit::TestRunner;

/// Run unit tests for 6502 assembly code
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the test YAML file
    #[arg(short, long)]
    input: PathBuf,

    /// Build directory for compiler output
    #[arg(short, long, help = "Directory for build outputs (required unless SOFT65C02_BUILD_DIR is set)")]
    build_dir: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Print commands that would be executed without actually running them
    #[arg(long)]
    dry_run: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    let runner = TestRunner::from_yaml(&cli.input, cli.build_dir, cli.verbose, cli.dry_run)?;
    runner.run()
}
