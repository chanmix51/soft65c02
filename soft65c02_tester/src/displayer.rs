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
    T: Write + Sync + Send,
{
    fn display(&mut self, receiver: Receiver<OutputToken>) -> AppResult<()> {
        let mut i: u32 = 0;

        while let Ok(token) = receiver.recv() {
            match token {
                OutputToken::Assertion {
                    failure,
                    description,
                } => {
                    i += 1;
                    self.output.write_all(
                        format!(
                            "⚡ {i:02} → {description} {}\n",
                            match failure {
                                None => "✅".to_string(),
                                Some(msg) => format!("❌ ({msg})"),
                            }
                        )
                        .as_bytes(),
                    )?;
                }
                OutputToken::Marker { description } => {
                    self.output
                        .write_all(format!("📄 {description}\n").as_bytes())?;
                }
                OutputToken::Run { loglines } if self.verbose => {
                    let mut content = loglines
                        .iter()
                        .map(|l| format!("🚀 {l}"))
                        .collect::<Vec<_>>()
                        .join("\n");
                    content.push('\n');
                    
                    if loglines.len() > 1 {
                        let total_cycles: u32 = loglines.iter().map(|l| l.cycles as u32).sum();
                        content.push_str(&format!("🕒 Total cycles: {}\n", total_cycles));
                    }
                    
                    self.output.write_all(content.as_bytes())?;
                }
                OutputToken::Setup(lines) if self.verbose => {
                    self.output
                        .write_all(format!("🔧 Setup: {}\n", lines[0]).as_bytes())?;
                }
                _ => (),
            }
        }

        Ok(())
    }
}
