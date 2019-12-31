extern crate minifb;

mod memory;
mod registers;
mod addressing_mode;
mod cpu_instruction;
mod processing_unit;

const INIT_VECTOR:usize = 0xFFFC;
const INTERRUPT_VECTOR:usize = 0xFFFE;

use memory::RAM as Memory;
use memory::AddressableIO;
use registers::Registers;
use processing_unit::*;
use cpu_instruction::{LogLine, CPUInstruction, MicrocodeError};

fn mem_dump(start: usize, end: usize, memory: &Memory) {
    let mut line = String::new();
    let address = start;
    let bytes = memory.read(start, end - start + 1).unwrap();

    while address < end {
        if address % 16 == start % 16 {
            println!("{}", line);
            line = format!("#{:04X}: ", address);
        } else if address % 8 == start % 8 {
            line = format!("{} ", line);
        }

        line = format!("{} {:02x}", line, bytes[address]);
    }

    println!("{}", line);
}

pub fn execute(memory: &mut Memory, registers: &mut Registers) -> Result<Vec<LogLine>, MicrocodeError> {
    let mut logs:Vec<LogLine> = vec![];

    loop {
        let cp = registers.command_pointer;
        match processing_unit::execute_step(registers, memory) {
            Ok(v)  => logs.push(v),
            Err(v) => break Err(v),
        }

        if registers.command_pointer == cp {
            break Ok(logs);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_program() {
        let init_vector:usize = 0x0800;
        let mut memory = memory::RAM::new();
        memory.write(init_vector, vec![0x48, 0xa9, 0x01, 0x8d, 0x00, 0x02, 0x6c, 0x00, 0x02, 0x95, 0x20, 0xa1, 0x20, 0x51, 0x21, 0x96, 0x21, 0x7d, 0x01, 0x02, 0xf9, 0x10, 0x12, 0xd0, 0xf6]).unwrap();
        let expected_output:Vec<&str> = vec![
            "#0x0800: (48)          PHA",
            "#0x0801: (a9 01)       LDA  #$01",
            "#0x0803: (8d 00 02)    STA  $0200",
            "#0x0806: (6c 00 02)    JMP  ($0200)",
            "#0x0809: (95 20)       STA  $20,X",
            "#0x080B: (a1 20)       LDA  ($20,X)",
            "#0x080D: (51 21)       EOR  ($21),Y",
            "#0x080F: (96 21)       STX  $21,Y",
            "#0x0811: (7d 01 02)    ADC  $0201,X",
            "#0x0814: (f9 10 12)    SBC  $1210,Y",
            "#0x0817: (d0 f6)       BNE  ±$f6",
            "#0x0819: (00)          BRK"
        ];
        let mut count:usize = 0;
        let output = disassemble(init_vector, 0x0819, &memory);

        for line in output {
            assert_eq!(format!("{}", expected_output[count]), format!("{}", line).as_str().trim().to_owned());
            count = count + 1;
        }
    }

    #[test]
    fn execute_program() {
        let init_vector:usize = 0x0800;
        let mut memory = memory::RAM::new();
        memory.write(init_vector, vec![0xa9, 0xc0, 0xaa, 0xe8, 0x69, 0x14, 0x00]).unwrap();
        let mut registers = Registers::new(init_vector);
        let loglines = execute(&mut memory, &mut registers).unwrap();
        let expected_output:Vec<&str> = vec![
            "#0x0800: (a9 c0)       LDA  #$c0     (#0x0801)",
            "#0x0802: (aa)          TAX",
            "#0x0803: (e8)          INX",
            "#0x0804: (69 14)       ADC  #$14     (#0x0805)",
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
}
