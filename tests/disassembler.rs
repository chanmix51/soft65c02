#[test]
fn read_program() {
    use soft65c02::{disassemble, AddressableIO, Memory};

    let init_vector: usize = 0x0800;
    let mut memory = Memory::new_with_ram();
    memory
        .write(
            init_vector,
            &vec![
                0xa9, 0xc0, 0xaa, 0xe8, 0x69, 0x14, 0x00, 0x3a, 0xd5, 0x20, 0xd0, 0xfe, 0xdb,
            ],
        )
        .unwrap();
    memory.write(0xfffe, &vec![0x00, 0x80]).unwrap();
    memory.write(0x8000, &vec![0x95, 0x20, 0x40]).unwrap();
    let expected_output: Vec<&str> = vec![
        "#0x0800: (a9 c0)       LDA  #$c0",
        "#0x0802: (aa)          TAX",
        "#0x0803: (e8)          INX",
        "#0x0804: (69 14)       ADC  #$14",
        "#0x0806: (00)          BRK",
        "#0x0807: (3a)          DEC  A",
        "#0x0808: (d5 20)       CMP  $20,X",
        "#0x080A: (d0 fe)       BNE  $080A",
        "#0x080C: (db)          STP",
    ];
    let mut count: usize = 0;
    let output = disassemble(init_vector, 0x080d, &memory).unwrap();

    for line in output {
        assert_eq!(
            format!("{}", expected_output[count]),
            format!("{}", line).as_str().trim().to_owned()
        );
        count = count + 1;
    }
}
