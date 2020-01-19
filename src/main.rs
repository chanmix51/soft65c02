extern crate rustyline;

use rustyline::error::ReadlineError;
use rustyline::Editor;
use soft65c02::{AddressableIO, LogLine, Memory, Registers, VERSION};

fn main() {
    // `()` can be used when no completer is required
    {
        println!("Soft 65C02 simulator");
        println!("Version {}", VERSION);
    }
    let mut rl = Editor::<()>::new();
    if rl.load_history("history.txt").is_err() {
        println!("No previous history, creating 'history.txt' file.");
    }
    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(line.as_str());
                println!("{}", line);
                println!("{}", parse(line).unwrap());
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupt sent to the CPU.");
            }
            Err(ReadlineError::Eof) => {
                println!("Quit!");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt").unwrap();
}

fn parse(command: String) -> Result<String, String> {
    if command.as_str().trim() == "help" {
        Ok(help())
    } else {
        Err(format!("Unknown command '{}'.", command))
    }
}

fn help() -> String {
    let help = r##"
Available commands:
  help:         Display this help page.
  memory:       The memory module.
  cpu:          The CPU module.
  program:      Load programs from the filesystem into memory.
  quit:         Quit Soft-65C02

Type <module> help to display each module's help page.
"##;
    help.to_owned()
}
