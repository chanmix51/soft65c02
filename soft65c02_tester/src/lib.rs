mod commands;
mod displayer;
mod executor;
mod pest_parser;
mod until_condition;
pub mod atari_binary;

pub use commands::*;
pub use displayer::*;
pub use executor::*;
pub use pest_parser::CliCommandParser;

pub type AppResult<T> = anyhow::Result<T>;
