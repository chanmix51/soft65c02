mod commands;
mod pest_parser;
mod until_condition;

use anyhow::Result;

pub type AppResult<T> = anyhow::Result<T>;

fn main() -> Result<()> {
    println!("Hello, world!");

    Ok(())
}
