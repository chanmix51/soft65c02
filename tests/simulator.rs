#[test]
fn execute_program() {
    use soft65c02::{Memory, Registers, AddressableIO, LogLine, execute};
    let init_vector:usize = 0x0800;
    let mut memory = Memory::new_with_ram();
    memory.write(init_vector, vec![0xa9, 0xc0, 0xaa, 0xe8, 0x69, 0x14, 0x00]).unwrap();
    let mut registers = Registers::new(init_vector);
    let loglines = execute(&mut memory, &mut registers).unwrap();
    let expected_output:Vec<&str> = vec![
        "#0x0800: (a9 c0)       LDA  #$c0     (#0x0801)  [A=0xc0][S=Nv-Bdizc]",
        "#0x0802: (aa)          TAX                      [X=0xc0][S=Nv-Bdizc]",
        "#0x0803: (e8)          INX                      [X=0xc1][S=Nv-Bdizc]",
        "#0x0804: (69 14)       ADC  #$14     (#0x0805)  [A=0xd4][S=Nv-Bdizc]",
        "#0x0806: (00)          BRK"
            ];
    let mut count:usize = 0;
    for line in loglines {
        assert_eq!(format!("{}", expected_output[count]), format!("{}", line).as_str().trim().to_owned());
        count += 1;
    }
    assert_eq!(0xc1, registers.register_x);
    assert_eq!(0xd4, registers.accumulator);
}

