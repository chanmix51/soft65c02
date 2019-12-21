use super::registers::Registers;
use super::memory::RAM as Memory;
use std::fmt;

pub struct AddressingModeResolution {
    pub operands:       Vec<u8>,
    pub addressing_mode:    AddressingMode,
    pub target_address:     Option<usize>,
}

impl AddressingModeResolution {
    fn new(operands: Vec<u8>, addressing_mode: AddressingMode, target_address: Option<usize>) -> Self {
        AddressingModeResolution {
            operands: operands,
            addressing_mode: addressing_mode,
            target_address: target_address,
        }
    }
}

impl fmt::Display for AddressingModeResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.addressing_mode {
            Implied  => write!(f, ""),
            ZeroPage => write!(f, "${:02x}", self.target_address.unwrap()),
            _ => panic!("Unsupported Display::fmt"),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum AddressingMode {
    Implied,
    ZeroPage,
}

impl AddressingMode {
    pub fn solve(&self, opcode_address: usize, memory: &Memory, registers: &Registers) -> AddressingModeResolution {
        match *self {
            AddressingMode::Implied  => AddressingModeResolution::new(vec![], self.clone(), None),
            AddressingMode::ZeroPage => {
                let byte = memory.read(opcode_address + 1, 1).unwrap()[0];
                AddressingModeResolution::new(vec![byte], self.clone(), Some(byte as usize))
            },
            _   => panic!("Can not solve this AddressingMode!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implied() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xe8, 0xff, 0xff]);
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Implied;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers);

        assert_eq!(0, resolution.operands.len());
        assert_eq!(None, resolution.target_address);
    }

    #[test]
    fn test_zero_page() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]);
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPage;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers);

        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0021, resolution.target_address.unwrap());
    }
}
