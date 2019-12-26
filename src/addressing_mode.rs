use super::registers::Registers;
use super::memory::RAM as Memory;
use super::memory::{little_endian, MemoryError};
use super::memory;
use std::error;
use std::fmt;

pub type Result<T> = std::result::Result<T, ResolutionError>;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum ResolutionError {
    Solving(AddressingMode, usize, Option<usize>),
    Memory(MemoryError),
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            ResolutionError::Solving(addressing_mode, opcode_address, target_address) => {
                let dst_addr_message = match target_address {
                    Some(v) => format!("#0x{:04X}", v),
                    None    => format!("none"),
                };

                write!(f, "resolution error for addressing mode '{}' for opcode at address #0x{:04X}, resolution result: {}", addressing_mode, opcode_address, dst_addr_message)
            },
            ResolutionError::Memory(e) => write!(f, "memory error during addressing mode resolution: {}", e),
        }
    }
}

impl error::Error for ResolutionError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl std::convert::From<memory::MemoryError> for ResolutionError {
    fn from(err: memory::MemoryError) -> ResolutionError {
        ResolutionError::Memory(err)
    }
}

#[derive(Debug)]
pub struct AddressingModeResolution {
    pub operands:           Vec<u8>,
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
            AddressingMode::Implied  => write!(f, ""),
            AddressingMode::Immediate  => write!(f, "#${:02x}", self.operands[0]),
            AddressingMode::ZeroPage => write!(f, "${:02x}", self.operands[0]),
            AddressingMode::Absolute => write!(f, "${:04x}", self.target_address.unwrap()),
            AddressingMode::AbsoluteXIndexed => write!(f, "${:04x},X", little_endian(self.operands.clone())),
            AddressingMode::AbsoluteYIndexed => write!(f, "${:04x},Y", little_endian(self.operands.clone())),
            AddressingMode::Indirect => write!(f, "(${:04x})", little_endian(self.operands.clone())),
            AddressingMode::ZeroPageXIndexed => write!(f, "${:02x},X", self.operands[0]),
            AddressingMode::ZeroPageYIndexed => write!(f, "${:02x},Y", self.operands[0]),
            AddressingMode::ZeroPageXIndexedIndirect => write!(f, "(${:02x},X)", self.operands[0]),
            AddressingMode::ZeroPageIndirectYIndexed => write!(f, "(${:02x}),Y", self.operands[0]),
            AddressingMode::Relative  => {
                let offset =  i8::from_ne_bytes(self.operands[0].to_ne_bytes());
                write!(f, "{}", offset)
            },
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum AddressingMode {
    Implied,
    Immediate,
    ZeroPage,
    ZeroPageXIndexed,
    ZeroPageYIndexed,
    ZeroPageXIndexedIndirect,
    ZeroPageIndirectYIndexed,
    Absolute,
    AbsoluteXIndexed,
    AbsoluteYIndexed,
    Indirect,
    Relative,
}

impl AddressingMode {
    /*
     * solve
     * Create a AddressingModeResolution using the memory and/or registers for
     * each AddressingMode.
     */
    pub fn solve(&self, opcode_address: usize, memory: &Memory, registers: &Registers) -> Result<AddressingModeResolution> {
        match *self {
            AddressingMode::Implied  => Ok(AddressingModeResolution::new(vec![], self.clone(), None)),
            AddressingMode::Immediate => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(opcode_address + 1 as usize)))
            },
            AddressingMode::ZeroPage => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                let byte = bytes[0];
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(byte as usize)))
            },
            AddressingMode::ZeroPageXIndexed => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                let byte = bytes[0];
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some((byte + registers.register_x) as usize)))
            },
            AddressingMode::ZeroPageYIndexed => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                let byte = bytes[0];
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some((byte + registers.register_y) as usize)))
            },
            AddressingMode::ZeroPageXIndexedIndirect => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                let dst_addr = little_endian(memory.read((bytes[0] + registers.register_x) as usize, 2)?);

                if dst_addr > memory::MEMMAX {
                    Err(ResolutionError::Solving(*self, opcode_address, Some(dst_addr)))
                } else {
                    Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dst_addr)))
                }
            },
            AddressingMode::ZeroPageIndirectYIndexed => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                let dst_addr = little_endian(memory.read(bytes[0] as usize, 2)?) + registers.register_y as usize;

                if dst_addr > memory::MEMMAX {
                    Err(ResolutionError::Solving(*self, opcode_address, Some(dst_addr)))
                } else {
                    Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dst_addr)))
                }
            },
            AddressingMode::Absolute => {
                let bytes = memory.read(opcode_address + 1, 2)?;
                let dest_addr = little_endian(bytes.clone());
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dest_addr)))
            },
            AddressingMode::AbsoluteXIndexed => {
                let bytes = memory.read(opcode_address + 1, 2)?;
                let dest_addr = little_endian(bytes.clone()) + registers.register_x as usize;
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dest_addr)))
            },
            AddressingMode::AbsoluteYIndexed => {
                let bytes = memory.read(opcode_address + 1, 2)?;
                let dest_addr = little_endian(bytes.clone()) + registers.register_y as usize;
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dest_addr)))
            },
            AddressingMode::Indirect => {
                let bytes = memory.read(opcode_address + 1, 2)?;
                let dst_addr = little_endian(memory.read(little_endian(bytes.clone()), 2)?);
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dst_addr)))
            },
            AddressingMode::Relative => {
                let bytes = memory.read(opcode_address + 1, 1)?;
                let dst_addr = {
                   let offset_i8  = i8::from_le_bytes(bytes[0].to_le_bytes());
                    if offset_i8 < 0 {
                        opcode_address.checked_sub( (0 - offset_i8) as usize)
                    } else {
                        opcode_address.checked_add(offset_i8 as usize)
                    }
                };

                if None == dst_addr {
                    return Err(ResolutionError::Solving(*self, opcode_address, None));
                }

                Ok(AddressingModeResolution::new(bytes, self.clone(), dst_addr))
            },
        }
    }
}

impl fmt::Display for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            AddressingMode::Implied  => write!(f, "implied"),
            AddressingMode::Immediate  => write!(f, "immediate"),
            AddressingMode::ZeroPage => write!(f, "zero page"),
            AddressingMode::Absolute => write!(f, "absolute"),
            AddressingMode::AbsoluteXIndexed => write!(f, "absolute X indexed"),
            AddressingMode::AbsoluteYIndexed => write!(f, "absolute Y indexed"),
            AddressingMode::Indirect => write!(f, "indirect"),
            AddressingMode::ZeroPageXIndexed => write!(f, "zero page X indexed"),
            AddressingMode::ZeroPageYIndexed => write!(f, "zero page Y indexed"),
            AddressingMode::ZeroPageXIndexedIndirect => write!(f, "zero page X indexed indirect"),
            AddressingMode::ZeroPageIndirectYIndexed => write!(f, "zero page indirect Y indexed"),
            AddressingMode::Relative  => write!(f, "relative"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implied() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xe8, 0xff, 0xff]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Implied;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(0, resolution.operands.len());
        assert_eq!(None, resolution.target_address);
        assert_eq!("".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_immediate() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xe8, 0xff, 0xff]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Immediate;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0xff], resolution.operands);
        assert_eq!(0x1001, resolution.target_address.unwrap());
        assert_eq!("#$ff".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPage;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0021, resolution.target_address.unwrap());
        assert_eq!("$21".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Absolute;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2221, resolution.target_address.unwrap());
        assert_eq!("$2221".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_x_indexed() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x05;
        let am = AddressingMode::AbsoluteXIndexed;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2226, resolution.target_address.unwrap());
        assert_eq!("$2221,X".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_y_indexed() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x16;
        let am = AddressingMode::AbsoluteYIndexed;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2237, resolution.target_address.unwrap());
        assert_eq!("$2221,Y".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_indirect() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x2221, vec![0x0a, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Indirect;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x800a, resolution.target_address.unwrap());
        assert_eq!("($2221)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_x_indexed() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x05;
        let am = AddressingMode::ZeroPageXIndexed;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0026, resolution.target_address.unwrap());
        assert_eq!("$21,X".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_y_indexed() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x05;
        let am = AddressingMode::ZeroPageYIndexed;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0026, resolution.target_address.unwrap());
        assert_eq!("$21,Y".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_indirect_y_indexed() {
        let mut memory = Memory::new();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x0021, vec![0x05, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x05;
        let am = AddressingMode::ZeroPageIndirectYIndexed;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();

        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x800a, resolution.target_address.unwrap());
        assert_eq!("($21),Y".to_owned(), format!("{}", resolution));
    }
}
