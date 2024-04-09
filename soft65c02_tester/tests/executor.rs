use std::{sync::mpsc::channel, thread::spawn};

use soft65c02_tester::{Executor, OutputToken};

#[test]
fn test_script() {
    let script = r#"marker $$first test$$
// main program
memory write #0x0800 0x(a9,c0,aa,e8,69,14,00,3a,d5,20,d0,fe,db)

// interrupt subroutine
memory write #0x8000 0x(95,20,40)

// set init vector
memory write #0xfffc 0x(00,08)

// set interrupt vector
memory write #0xfffe 0x(00,80)

// test
run #0x1000 until CP=0x8000
assert A=0xc0 $$accumulator is loaded$$
run until CP=0x080A
run
assert CP=0x080C $$command pointer points at EOP$$"#;
    let lines: Vec<&str> = script.split('\n').collect();
    let executor = Executor::new(&lines).unwrap();
    let (sender, receiver) = channel::<OutputToken>();
    let handler = spawn(move || {
        let mut i: u32 = 0;

        while let Ok(token) = receiver.recv() {
            match token {
                OutputToken::Assertion {
                    success,
                    description,
                } => {
                    i += 1;
                    println!(
                        "{i:02} → {description} {}",
                        if success { "✅" } else { "❌" }
                    );
                }
                OutputToken::Marker { description } => {
                    println!("♯ {description}")
                }
                _ => {}
            }
        }
    });
    executor.run(sender).unwrap();
    handler.join().unwrap();
}
