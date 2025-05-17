use soft65c02_lib::{execute_step, AddressableIO, CPUError, LogLine, Memory, Registers};

fn execute(memory: &mut Memory, registers: &mut Registers) -> Result<Vec<LogLine>, CPUError> {
    let mut cp: usize = 0;
    let mut output: Vec<LogLine> = vec![];

    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        output.push(execute_step(registers, memory)?);
    }

    Ok(output)
}

#[test]
fn execute_program() {
    let init_vector: usize = 0x0800;
    let mut memory = Memory::new_with_ram();
    memory
        .write(
            init_vector,
            &[
                0xa9, 0xc0, 0xaa, 0xe8, 0x69, 0x14, 0x00, 0x3a, 0xd5, 0x20, 0xd0, 0xfe, 0xdb,
            ],
        )
        .unwrap();
    memory.write(0xfffe, &[0x00, 0x80]).unwrap();
    memory.write(0x8000, &[0x95, 0x20, 0x40]).unwrap();
    let mut registers = Registers::new_initialized(init_vector);
    let loglines = execute(&mut memory, &mut registers).unwrap();
    let expected_output: Vec<&str> = vec![
        "#0x0800: (a9 c0)       LDA  #$c0     (#0x0801)  [A=0xc0][S=Nv-Bdizc][2]",
        "#0x0802: (aa)          TAX                      [X=0xc0][S=Nv-Bdizc][2]",
        "#0x0803: (e8)          INX                      [X=0xc1][S=Nv-Bdizc][2]",
        "#0x0804: (69 14)       ADC  #$14     (#0x0805)  (0x14)[A=0xd4][S=Nv-Bdizc][2]",
        "#0x0806: (00)          BRK                      [CP=0x8000][SP=0xfc][S=Nv-BdIzc][7]",
        "#0x8000: (95 20)       STA  $20,X    (#0x00E1)  (0xd4)[4]",
        "#0x8002: (40)          RTI                      [CP=0x0808][SP=0xff][S=Nv-Bdizc][6]",
        "#0x0808: (d5 20)       CMP  $20,X    (#0x00E1)  (0xd4)[A=0xd4][S=nv-BdiZC][4]",
        "#0x080A: (d0 fe)       BNE  $080A               [CP=0x080C][2]",
        "#0x080C: (db)          STP                      [S=nv-BdiZC][3]",
    ];
    loglines.iter().enumerate().for_each(|(i, line)| {
        assert_eq!(
            format!("{}", expected_output[i]),
            format!("{}", line).as_str().trim().to_owned()
        )
    });
    assert_eq!(0xc1, registers.register_x);
    assert_eq!(0xd4, registers.accumulator);
}
