use std::{io::Write, sync::mpsc::Receiver};

use crate::{AppResult, OutputToken};

pub trait Displayer {
    fn display(&mut self, receiver: Receiver<OutputToken>) -> AppResult<()>;
}

#[derive(Debug, Default)]
pub struct CliDisplayer<T>
where
    T: Write,
{
    output: T,
    verbose: bool,
}

impl<T> CliDisplayer<T>
where
    T: Write,
{
    pub fn new(output: T, verbose: bool) -> Self {
        Self { output, verbose }
    }
}

impl<T> Displayer for CliDisplayer<T>
where
    T: Write,
{
    fn display(&mut self, receiver: Receiver<OutputToken>) -> AppResult<()> {
        let mut i: u32 = 0;

        while let Ok(token) = receiver.recv() {
            match token {
                OutputToken::Assertion {
                    success,
                    description,
                } => {
                    i += 1;
                    self.output.write_all(
                        format!(
                            "{i:02} → {description} {}\n",
                            if success { "✅" } else { "❌" }
                        )
                        .as_bytes(),
                    )?;
                }
                OutputToken::Marker { description } => {
                    self.output
                        .write_all(format!("♯ {description}\n").as_bytes())?;
                }
                OutputToken::Run { loglines } if self.verbose => {
                    let mut content = loglines
                        .iter()
                        .map(|l| format!("⚡ {l}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    content.push('\n');
                    self.output.write_all(content.as_bytes())?;
                }
                OutputToken::Setup(_lines) if self.verbose => {
                    todo!()
                }
                _ => (),
            }
        }

        Ok(())
    }
}
