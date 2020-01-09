use super::registers::Registers;
use super::memory::MemoryStack as Memory;
use super::memory::{little_endian, MemoryError, AddressableIO};
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
        match self.target_address {
            Some(addr)  => write!(f, "{: <9}(#0x{:04X})", format!("{}", self.addressing_mode), addr),
            None        => write!(f, "{: <9}         ", format!("{}", self.addressing_mode)),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum AddressingMode {
    Implied,
    Accumulator,
    Immediate([u8; 1]),
    ZeroPage([u8; 1]),
    ZeroPageXIndexed([u8; 1]),
    ZeroPageYIndexed([u8; 1]),
    ZeroPageXIndexedIndirect([u8; 1]),
    ZeroPageIndirectYIndexed([u8; 1]),
    ZeroPageIndirect([u8; 1]),
    Absolute([u8; 2]),
    AbsoluteXIndexed([u8; 2]),
    AbsoluteXIndexedIndirect([u8; 2]),
    AbsoluteYIndexed([u8; 2]),
    Indirect([u8; 2]),
    Relative([u8; 1]),
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
            AddressingMode::Accumulator  => Ok(AddressingModeResolution::new(vec![], self.clone(), None)),
            AddressingMode::Immediate(v) => {
                Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(opcode_address + 1 as usize)))
            },
            AddressingMode::ZeroPage(v) => {
                Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(v[0] as usize)))
            },
            AddressingMode::ZeroPageXIndexed(v) => {
                Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(v[0] as usize + registers.register_x as usize )))
            },
            AddressingMode::ZeroPageYIndexed(v) => {
                Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(v[0] as usize + registers.register_y as usize)))
            },
            AddressingMode::ZeroPageXIndexedIndirect(v) => {
                let dst_addr = little_endian(memory.read((v[0] + registers.register_x) as usize, 2)?);

                if dst_addr > memory::MEMMAX {
                    Err(ResolutionError::Solving(*self, opcode_address, Some(dst_addr)))
                } else {
                    Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(dst_addr)))
                }
            },
            AddressingMode::ZeroPageIndirectYIndexed(v) => {
                let dst_addr = little_endian(memory.read(v[0] as usize, 2)?) + registers.register_y as usize;

                if dst_addr > memory::MEMMAX {
                    Err(ResolutionError::Solving(*self, opcode_address, Some(dst_addr)))
                } else {
                    Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(dst_addr)))
                }
            },
            AddressingMode::ZeroPageIndirect(v) => {
                let dst_addr = little_endian(memory.read(v[0] as usize, 2)?);
                Ok(AddressingModeResolution::new(vec![v[0]], self.clone(), Some(dst_addr)))
            },
            AddressingMode::Absolute(v) => {
                let dest_addr = little_endian(vec![v[0], v[1]]);
                Ok(AddressingModeResolution::new(vec![v[0],v[1]], self.clone(), Some(dest_addr)))
            },
            AddressingMode::AbsoluteXIndexed(v) => {
                let bytes = vec![v[0], v[1]];
                let dest_addr = little_endian(bytes.clone()) + registers.register_x as usize;
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dest_addr)))
            },
            AddressingMode::AbsoluteXIndexedIndirect(v) => {
                let bytes = vec![v[0], v[1]];
                let tmp_addr = little_endian(bytes.clone()) + registers.register_x as usize;
                let dest_addr = little_endian(memory.read(tmp_addr, 2)?);
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dest_addr)))
            },
            AddressingMode::AbsoluteYIndexed(v) => {
                let bytes = vec![v[0], v[1]];
                let dest_addr = little_endian(bytes.clone()) + registers.register_y as usize;
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dest_addr)))
            },
            AddressingMode::Indirect(v) => {
                let bytes = vec![v[0], v[1]];
                let dst_addr = little_endian(memory.read(little_endian(bytes.clone()), 2)?);
                Ok(AddressingModeResolution::new(bytes, self.clone(), Some(dst_addr)))
            },
            AddressingMode::Relative(v) => {
                let bytes = vec![v[0]];
                let dst_addr = {
                   let offset_i8  = i8::from_le_bytes(bytes[0].to_le_bytes());
                    if offset_i8 < 0 {
                        opcode_address.checked_sub( (-2 - offset_i8) as usize)
                    } else {
                        opcode_address.checked_add((offset_i8  as usize) + 2)
                    }
                };

                if None == dst_addr {
                    return Err(ResolutionError::Solving(*self, opcode_address, None));
                }

                Ok(AddressingModeResolution::new(bytes, self.clone(), dst_addr))
            },
        }
    }

    pub fn get_operands(&self) -> Vec<u8> {
        match *self {
            AddressingMode::Implied => vec![],
            AddressingMode::Accumulator => vec![],
            AddressingMode::Immediate(v) => v.to_vec(),
            AddressingMode::ZeroPage(v) => v.to_vec(),
            AddressingMode::ZeroPageXIndexed(v) => v.to_vec(),
            AddressingMode::ZeroPageYIndexed(v) => v.to_vec(),
            AddressingMode::ZeroPageXIndexedIndirect(v) => v.to_vec(),
            AddressingMode::ZeroPageIndirectYIndexed(v) => v.to_vec(),
            AddressingMode::ZeroPageIndirect(v) => v.to_vec(),
            AddressingMode::Absolute(v) => v.to_vec(),
            AddressingMode::AbsoluteXIndexed(v) => v.to_vec(),
            AddressingMode::AbsoluteXIndexedIndirect(v) => v.to_vec(),
            AddressingMode::AbsoluteYIndexed(v) => v.to_vec(),
            AddressingMode::Indirect(v) => v.to_vec(),
            AddressingMode::Relative(v) => v.to_vec(),
        }
    }
}

impl fmt::Display for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            AddressingMode::Implied  => write!(f, ""),
            AddressingMode::Accumulator  => write!(f, "A"),
            AddressingMode::Immediate(v)  => write!(f, "#${:02x}", v[0]),
            AddressingMode::ZeroPage(v) => write!(f, "${:02x}", v[0]),
            AddressingMode::Absolute(v) => write!(f, "${:02X}{:02X}", v[1], v[0]),
            AddressingMode::AbsoluteXIndexed(v) => write!(f, "${:02X}{:02X},X", v[1], v[0]),
            AddressingMode::AbsoluteXIndexedIndirect(v) => write!(f, "(${:02X}{:02X},X)", v[1], v[0]),
            AddressingMode::AbsoluteYIndexed(v) => write!(f, "${:02X}{:02X},Y", v[1], v[0]),
            AddressingMode::Indirect(v) => write!(f, "(${:02X}{:02X})", v[1], v[0]),
            AddressingMode::ZeroPageXIndexed(v) => write!(f, "${:02x},X", v[0]),
            AddressingMode::ZeroPageYIndexed(v) => write!(f, "${:02x},Y", v[0]),
            AddressingMode::ZeroPageXIndexedIndirect(v) => write!(f, "(${:02x},X)", v[0]),
            AddressingMode::ZeroPageIndirectYIndexed(v) => write!(f, "(${:02x}),Y", v[0]),
            AddressingMode::ZeroPageIndirect(v) => write!(f, "(${:02x})", v[0]),
            AddressingMode::Relative(v)  => write!(f, "{: <+2}", { i8::from_le_bytes(u8::to_le_bytes(v[0])) }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implied() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xe8, 0xff, 0xff]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Implied;
        assert_eq!("".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(0, resolution.operands.len());
        assert_eq!(None, resolution.target_address);
        assert_eq!("", format!("{}", resolution).as_str().trim());
    }

    #[test]
    fn test_accumulator() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xe8, 0xff, 0xff]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Accumulator;
        assert_eq!("A".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(0, resolution.operands.len());
        assert_eq!(None, resolution.target_address);
        assert_eq!("A", format!("{}", resolution).as_str().trim());
    }

    #[test]
    fn test_immediate() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xe8, 0xff, 0xff]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Immediate([0xff]);
        assert_eq!("#$ff".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0xff], resolution.operands);
        assert_eq!(0x1001, resolution.target_address.unwrap());
        assert_eq!("#$ff     (#0x1001)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPage([0x21]);
        assert_eq!("$21".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0021, resolution.target_address.unwrap());
        assert_eq!("$21      (#0x0021)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x2a]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Absolute([0x21, 0x2a]);
        assert_eq!("$2A21".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21, 0x2a], resolution.operands);
        assert_eq!(0x2a21, resolution.target_address.unwrap());
        assert_eq!("$2A21    (#0x2A21)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_x_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x05;
        let am = AddressingMode::AbsoluteXIndexed([0x21, 0x22]);
        assert_eq!("$2221,X".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2226, resolution.target_address.unwrap());
        assert_eq!("$2221,X  (#0x2226)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_y_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x16;
        let am = AddressingMode::AbsoluteYIndexed([0x21, 0x22]);
        assert_eq!("$2221,Y".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2237, resolution.target_address.unwrap());
        assert_eq!("$2221,Y  (#0x2237)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_indirect() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x2221, vec![0x0a, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Indirect([0x21, 0x22]);
        assert_eq!("($2221)".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x800a, resolution.target_address.unwrap());
        assert_eq!("($2221)  (#0x800A)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_x_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x05;
        let am = AddressingMode::ZeroPageXIndexed([0x21]);
        assert_eq!("$21,X".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0026, resolution.target_address.unwrap());
        assert_eq!("$21,X    (#0x0026)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_y_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x05;
        let am = AddressingMode::ZeroPageYIndexed([0x21]);
        assert_eq!("$21,Y".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0026, resolution.target_address.unwrap());
        assert_eq!("$21,Y    (#0x0026)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_indirect_y_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x0021, vec![0x05, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x05;
        let am = AddressingMode::ZeroPageIndirectYIndexed([0x21]);
        assert_eq!("($21),Y".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x800a, resolution.target_address.unwrap());
        assert_eq!("($21),Y  (#0x800A)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_positive() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xd0, 0x04, 0x22, 0x00, 0x12, 0x0a]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Relative([0x04]);
        assert_eq!("+4".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x04], resolution.operands);
        assert_eq!(0x1006, resolution.target_address.unwrap());
        assert_eq!("+4       (#0x1006)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_negative() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x0ffa, vec![0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0xd0, 0xfb, 0x22, 0x00, 0x12, 0x0a]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Relative([0xfb]);
        assert_eq!("-5".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0xfb], resolution.operands);
        assert_eq!(0x0ffd, resolution.target_address.unwrap());
        assert_eq!("-5       (#0x0FFD)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_negative_edge() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x0ffa, vec![0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0xd0, 0x80, 0x22, 0x00, 0x12, 0x0a]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Relative([0x80]);
        assert_eq!("-128".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x80], resolution.operands);
        assert_eq!(0x0F82, resolution.target_address.unwrap());
        assert_eq!("-128     (#0x0F82)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_positive_edge() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x0ffa, vec![0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0xd0, 0x7f, 0x22, 0x00, 0x12, 0x0a]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::Relative([0x7f]);
        assert_eq!("+127".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x7f], resolution.operands);
        assert_eq!(0x1081, resolution.target_address.unwrap());
        assert_eq!("+127     (#0x1081)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_indirect() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x0021, vec![0x05, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPageIndirect([0x21]);
        assert_eq!("($21)".to_owned(), format!("{}", am));

        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x8005, resolution.target_address.unwrap());
        assert_eq!("($21)    (#0x8005)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_x_indexed_indirect() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, vec![0xa5, 0x21, 0x20]).unwrap();
        memory.write(0x2025, vec![0x05, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::AbsoluteXIndexedIndirect([0x21, 0x20]);
        assert_eq!("($2021,X)".to_owned(), format!("{}", am));

        registers.register_x = 0x04;
        let resolution:AddressingModeResolution = am.solve(0x1000, &mut memory, &mut registers).unwrap();
        assert_eq!(vec![0x21, 0x20], resolution.operands);
        assert_eq!(0x8005, resolution.target_address.unwrap());
        assert_eq!("($2021,X)(#0x8005)".to_owned(), format!("{}", resolution));
    }
}
