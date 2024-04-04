mod commands;
mod pest_parser;
mod until_condition;

pub use commands::*;
pub use pest_parser::CliCommandParser;

pub type AppResult<T> = anyhow::Result<T>;
