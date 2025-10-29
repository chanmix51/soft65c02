use std::{io::Cursor, sync::mpsc::channel};

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
run init until CP=0x8000
assert A=0xc0 $$accumulator is loaded$$
run until CP=0x080A
run
assert CP=0x080C $$command pointer points at EOP$$"#;
    let executor = Executor::default();
    let (sender, receiver) = channel::<OutputToken>();
    executor.run(Cursor::new(script), sender).unwrap_err();

    let token = receiver.recv().unwrap();
    assert!(matches!(
        token,
        OutputToken::Marker {
            description } if description == *"first test"
    ));
    let token = receiver.recv().unwrap();
    assert!(matches!(
        token,
        OutputToken::Setup(lines) if lines == vec!["13 bytes written"]
    ));
    let token = receiver.recv().unwrap();
    assert!(matches!(
        token,
        OutputToken::Setup(lines) if lines == vec!["3 bytes written"]
    ));
    let token = receiver.recv().unwrap();
    assert!(matches!(
        token,
        OutputToken::Setup(lines) if lines == vec!["2 bytes written"]
    ));
    let token = receiver.recv().unwrap();
    assert!(matches!(
        token,
        OutputToken::Setup(lines) if lines == vec!["2 bytes written"]
    ));
    let token = receiver.recv().unwrap();
    assert!(matches!(token, OutputToken::Run { loglines: _logs, symbols: None }));
    let token = receiver.recv().unwrap();

    assert!(
        matches!(token, OutputToken::Assertion { failure, description } if failure.is_some() && description == *"accumulator is loaded")
    );

    let _ = receiver.recv().unwrap_err();
}
