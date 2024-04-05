use soft65c02_lib::{Memory, Registers};

use soft65c02_tester::{CliCommandParser, Command};

#[test]
fn test_register_flush() {
    let mut registers = Registers::new(0x1234);
    let mut memory = Memory::new_with_ram();

    let output = CliCommandParser::from("registers flush")
        .unwrap()
        .execute(&mut registers, &mut memory)
        .unwrap();

    assert_eq!(0, output.len());
}
