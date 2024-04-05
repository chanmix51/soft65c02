use soft65c02_lib::{Memory, Registers};

use soft65c02_tester::{CliCommandParser, Command};

#[test]
fn test_assertion() {
    let mut registers = Registers::new(0x0000);
    let mut memory = Memory::new_with_ram();

    let output = CliCommandParser::from("assert #0x0000 = 0x00 $$The first byte is zero$$")
        .unwrap()
        .execute(&mut registers, &mut memory)
        .unwrap();

    assert_eq!("The first byte is zero".to_string(), output[0]);
}

#[test]
fn test_bad_assertion() {
    let mut registers = Registers::new(0x0000);
    let mut memory = Memory::new_with_ram();

    let output = CliCommandParser::from("assert #0x0000 = 0x01 $$The first byte is one, really?$$")
        .unwrap()
        .execute(&mut registers, &mut memory)
        .unwrap_err();

    assert_eq!(
        "The first byte is one, really?".to_string(),
        output.to_string()
    );
}
