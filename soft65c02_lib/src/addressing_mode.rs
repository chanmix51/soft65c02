use super::memory;
use super::memory::MemoryStack as Memory;
use super::memory::{little_endian, AddressableIO, MemoryError};
use super::registers::Registers;
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
                    Some(v) => format!("#0x{v:04X}"),
                    None => "none".to_owned(),
                };

                write!(f, "resolution error for addressing mode '{}' for opcode at address #0x{:04X}, resolution result: {}", addressing_mode, opcode_address, dst_addr_message)
            }
            ResolutionError::Memory(e) => {
                write!(f, "memory error during addressing mode resolution: {}", e)
            }
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

#[derive(Debug, Clone)]
pub struct AddressingModeResolution {
    pub operands: Vec<u8>,
    pub addressing_mode: AddressingMode,
    pub target_address: Option<usize>,
}

impl AddressingModeResolution {
    fn new(
        operands: Vec<u8>,
        addressing_mode: AddressingMode,
        target_address: Option<usize>,
    ) -> Self {
        AddressingModeResolution {
            operands,
            addressing_mode,
            target_address,
        }
    }
}

impl fmt::Display for AddressingModeResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.target_address {
            Some(addr) => write!(
                f,
                "{: <9}(#0x{:04X})",
                format!("{}", self.addressing_mode),
                addr
            ),
            None => write!(f, "{: <9}         ", format!("{}", self.addressing_mode)),
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
    Relative(usize, [u8; 1]),
    ZeroPageRelative(usize, [u8; 2]),
}

impl AddressingMode {
    /*
     * solve
     * Create a AddressingModeResolution using the memory and/or registers for
     * each AddressingMode.
     */
    pub fn solve(
        &self,
        opcode_address: usize,
        memory: &Memory,
        registers: &Registers,
    ) -> Result<AddressingModeResolution> {
        match *self {
            AddressingMode::Implied => Ok(AddressingModeResolution::new(vec![], *self, None)),
            AddressingMode::Accumulator => Ok(AddressingModeResolution::new(vec![], *self, None)),
            AddressingMode::Immediate(v) => Ok(AddressingModeResolution::new(
                vec![v[0]],
                *self,
                Some(opcode_address + 1),
            )),
            AddressingMode::ZeroPage(v) => Ok(AddressingModeResolution::new(
                vec![v[0]],
                *self,
                Some(v[0] as usize),
            )),
            AddressingMode::ZeroPageXIndexed(v) => Ok(AddressingModeResolution::new(
                vec![v[0]],
                *self,
                Some((v[0] as usize + registers.register_x as usize) % 0x100),
            )),
            AddressingMode::ZeroPageYIndexed(v) => Ok(AddressingModeResolution::new(
                vec![v[0]],
                *self,
                Some((v[0] as usize + registers.register_y as usize) % 0x100),
            )),
            AddressingMode::ZeroPageXIndexedIndirect(v) => {
                let pointer_addr = (v[0] as usize + registers.register_x as usize) % 0x100;

                // Handle zero page wraparound for pointer reading
                let lsb = memory.read(pointer_addr, 1)?[0];
                let msb = memory.read((pointer_addr + 1) % 0x100, 1)?[0];
                let dst_addr = little_endian(vec![lsb, msb]);

                // Address should wrap at 16-bit boundary (like the real 6502/65C02)
                let wrapped_addr = dst_addr & 0xFFFF;
                Ok(AddressingModeResolution::new(
                    vec![v[0]],
                    *self,
                    Some(wrapped_addr),
                ))
            }
            AddressingMode::ZeroPageIndirectYIndexed(v) => {
                let pointer_addr = v[0] as usize;

                // Handle zero page wraparound for pointer reading
                let lsb = memory.read(pointer_addr, 1)?[0];
                let msb = memory.read((pointer_addr + 1) % 0x100, 1)?[0];
                let base_addr = little_endian(vec![lsb, msb]);
                let dst_addr = base_addr + registers.register_y as usize;

                // Address should wrap at 16-bit boundary (like the real 6502/65C02)
                let wrapped_addr = dst_addr & 0xFFFF;
                Ok(AddressingModeResolution::new(
                    vec![v[0]],
                    *self,
                    Some(wrapped_addr),
                ))
            }
            AddressingMode::ZeroPageIndirect(v) => {
                let pointer_addr = v[0] as usize;

                // Handle zero page wraparound for pointer reading
                let lsb = memory.read(pointer_addr, 1)?[0];
                let msb = memory.read((pointer_addr + 1) % 0x100, 1)?[0];
                let dst_addr = little_endian(vec![lsb, msb]);
                Ok(AddressingModeResolution::new(
                    vec![v[0]],
                    *self,
                    Some(dst_addr),
                ))
            }
            AddressingMode::Absolute(v) => {
                let dest_addr = little_endian(vec![v[0], v[1]]);
                Ok(AddressingModeResolution::new(
                    vec![v[0], v[1]],
                    *self,
                    Some(dest_addr),
                ))
            }
            AddressingMode::AbsoluteXIndexed(v) => {
                let bytes = vec![v[0], v[1]];
                let dest_addr = little_endian(bytes.clone()) + registers.register_x as usize;
                // Address should wrap at 16-bit boundary (like the real 6502/65C02)
                let wrapped_addr = dest_addr & 0xFFFF;
                Ok(AddressingModeResolution::new(bytes, *self, Some(wrapped_addr)))
            }
            AddressingMode::AbsoluteXIndexedIndirect(v) => {
                let bytes = vec![v[0], v[1]];
                let tmp_addr = little_endian(bytes.clone()) + registers.register_x as usize;
                // Intermediate address should wrap at 16-bit boundary
                let wrapped_tmp_addr = tmp_addr & 0xFFFF;
                let dest_addr = little_endian(memory.read(wrapped_tmp_addr, 2)?);
                // Final address should also wrap at 16-bit boundary
                let wrapped_dest_addr = dest_addr & 0xFFFF;
                Ok(AddressingModeResolution::new(bytes, *self, Some(wrapped_dest_addr)))
            }
            AddressingMode::AbsoluteYIndexed(v) => {
                let bytes = vec![v[0], v[1]];
                let dest_addr = little_endian(bytes.clone()) + registers.register_y as usize;
                // Address should wrap at 16-bit boundary (like the real 6502/65C02)
                let wrapped_addr = dest_addr & 0xFFFF;
                Ok(AddressingModeResolution::new(bytes, *self, Some(wrapped_addr)))
            }
            AddressingMode::Indirect(v) => {
                let bytes = vec![v[0], v[1]];
                let dst_addr = little_endian(memory.read(little_endian(bytes.clone()), 2)?);
                Ok(AddressingModeResolution::new(bytes, *self, Some(dst_addr)))
            }
            AddressingMode::Relative(_addr, v) => {
                let bytes = vec![v[0]];
                Ok(AddressingModeResolution::new(bytes, *self, None))
            }
            AddressingMode::ZeroPageRelative(_addr, v) => {
                let bytes = v.to_vec();
                let dst_addr = Some(bytes[0] as usize).ok_or(ResolutionError::Solving(
                    *self,
                    opcode_address,
                    None,
                ))?;

                Ok(AddressingModeResolution::new(bytes, *self, Some(dst_addr)))
            }
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
            AddressingMode::Relative(_addr, v) => v.to_vec(),
            AddressingMode::ZeroPageRelative(_addr, v) => v.to_vec(),
        }
    }

    fn crosses_page_boundary(&self, base_addr: usize, index: u8) -> bool {
        let base_page = (base_addr & 0xFF00) >> 8;
        let indexed_addr = base_addr.wrapping_add(index as usize);
        let indexed_page = (indexed_addr & 0xFF00) >> 8;
        base_page != indexed_page
    }

    pub fn needs_page_crossing_cycle(&self, registers: &Registers, memory: &Memory) -> bool {
        match self {
            // For indexed addressing modes, check if page boundary is crossed
            AddressingMode::AbsoluteXIndexed(v) => {
                let base_addr = little_endian(vec![v[0], v[1]]);
                self.crosses_page_boundary(base_addr, registers.register_x)
            }
            AddressingMode::AbsoluteYIndexed(v) => {
                let base_addr = little_endian(vec![v[0], v[1]]);
                self.crosses_page_boundary(base_addr, registers.register_y)
            }
            AddressingMode::ZeroPageIndirectYIndexed(v) => {
                if let Ok(bytes) = memory.read(v[0] as usize, 2) {
                    let base_addr = little_endian(bytes);
                    self.crosses_page_boundary(base_addr, registers.register_y)
                } else {
                    false
                }
            }
            AddressingMode::Relative(addr, offset) => {
                let next_instr = *addr + 2;
                let target = resolve_relative(*addr, offset[0]).unwrap_or(next_instr);
                (next_instr & 0xFF00) != (target & 0xFF00)
            }
            // All other addressing modes never incur page crossing penalties
            _ => false
        }
    }
}

impl fmt::Display for AddressingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            AddressingMode::Implied => write!(f, ""),
            AddressingMode::Accumulator => write!(f, "A"),
            AddressingMode::Immediate(v) => write!(f, "#${:02x}", v[0]),
            AddressingMode::ZeroPage(v) => write!(f, "${:02x}", v[0]),
            AddressingMode::Absolute(v) => write!(f, "${:02X}{:02X}", v[1], v[0]),
            AddressingMode::AbsoluteXIndexed(v) => write!(f, "${:02X}{:02X},X", v[1], v[0]),
            AddressingMode::AbsoluteXIndexedIndirect(v) => {
                write!(f, "(${:02X}{:02X},X)", v[1], v[0])
            }
            AddressingMode::AbsoluteYIndexed(v) => write!(f, "${:02X}{:02X},Y", v[1], v[0]),
            AddressingMode::Indirect(v) => write!(f, "(${:02X}{:02X})", v[1], v[0]),
            AddressingMode::ZeroPageXIndexed(v) => write!(f, "${:02x},X", v[0]),
            AddressingMode::ZeroPageYIndexed(v) => write!(f, "${:02x},Y", v[0]),
            AddressingMode::ZeroPageXIndexedIndirect(v) => write!(f, "(${:02x},X)", v[0]),
            AddressingMode::ZeroPageIndirectYIndexed(v) => write!(f, "(${:02x}),Y", v[0]),
            AddressingMode::ZeroPageIndirect(v) => write!(f, "(${:02x})", v[0]),
            AddressingMode::Relative(addr, v) => {
                write!(f, "${:04X}", resolve_relative(addr, v[0]).unwrap())
            }
            AddressingMode::ZeroPageRelative(addr, v) => {
                write!(
                    f,
                    "${:02x},${:04X}",
                    v[0],
                    resolve_relative(addr, v[1]).unwrap()
                )
            }
        }
    }
}

pub fn resolve_relative(addr: usize, offset: u8) -> Option<usize> {
    let offset_i8 = i8::from_le_bytes(offset.to_le_bytes());
    if offset_i8 < 0 {
        addr.checked_sub((-2 - offset_i8) as usize)
    } else {
        addr.checked_add((offset_i8 as usize) + 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implied() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xe8, 0xff, 0xff]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Implied;
        assert_eq!("".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(0, resolution.operands.len());
        assert_eq!(None, resolution.target_address);
        assert_eq!("", format!("{}", resolution).as_str().trim());
    }

    #[test]
    fn test_accumulator() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xe8, 0xff, 0xff]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Accumulator;
        assert_eq!("A".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(0, resolution.operands.len());
        assert_eq!(None, resolution.target_address);
        assert_eq!("A", format!("{}", resolution).as_str().trim());
    }

    #[test]
    fn test_immediate() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xe8, 0xff, 0xff]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Immediate([0xff]);
        assert_eq!("#$ff".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0xff], resolution.operands);
        assert_eq!(0x1001, resolution.target_address.unwrap());
        assert_eq!("#$ff     (#0x1001)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPage([0x21]);
        assert_eq!("$21".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0021, resolution.target_address.unwrap());
        assert_eq!("$21      (#0x0021)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x2a]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Absolute([0x21, 0x2a]);
        assert_eq!("$2A21".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21, 0x2a], resolution.operands);
        assert_eq!(0x2a21, resolution.target_address.unwrap());
        assert_eq!("$2A21    (#0x2A21)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_x_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x05;
        let am = AddressingMode::AbsoluteXIndexed([0x21, 0x22]);
        assert_eq!("$2221,X".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2226, resolution.target_address.unwrap());
        assert_eq!("$2221,X  (#0x2226)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_y_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x16;
        let am = AddressingMode::AbsoluteYIndexed([0x21, 0x22]);
        assert_eq!("$2221,Y".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x2237, resolution.target_address.unwrap());
        assert_eq!("$2221,Y  (#0x2237)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_indirect() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x2221, &[0x0a, 0x80]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Indirect([0x21, 0x22]);
        assert_eq!("($2221)".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21, 0x22], resolution.operands);
        assert_eq!(0x800a, resolution.target_address.unwrap());
        assert_eq!("($2221)  (#0x800A)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_x_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 0x05;
        let am = AddressingMode::ZeroPageXIndexed([0x21]);
        assert_eq!("$21,X".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0026, resolution.target_address.unwrap());
        assert_eq!("$21,X    (#0x0026)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_y_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x05;
        let am = AddressingMode::ZeroPageYIndexed([0x21]);
        assert_eq!("$21,Y".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x0026, resolution.target_address.unwrap());
        assert_eq!("$21,Y    (#0x0026)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_wraparound() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x15;
        let am = AddressingMode::ZeroPageYIndexed([0xeb]);
        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(0x0000, resolution.target_address.unwrap());
    }

    #[test]
    fn test_zero_page_indirect_y_indexed() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x0021, &[0x05, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 0x05;
        let am = AddressingMode::ZeroPageIndirectYIndexed([0x21]);
        assert_eq!("($21),Y".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x800a, resolution.target_address.unwrap());
        assert_eq!("($21),Y  (#0x800A)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_positive() {
        let mut memory = Memory::new_with_ram();
        memory
            .write(0x1000, &[0xd0, 0x04, 0x22, 0x00, 0x12, 0x0a])
            .unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Relative(0x1000, [0x04]);
        assert_eq!("$1006".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x04], resolution.operands);
        assert_eq!(None, resolution.target_address);
        assert_eq!("$1006             ".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_negative() {
        let mut memory = Memory::new_with_ram();
        memory
            .write(
                0x0ffa,
                &[
                    0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0xd0, 0xfb, 0x22, 0x00, 0x12, 0x0a,
                ],
            )
            .unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Relative(0x1000, [0xfb]);
        assert_eq!("$0FFD".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0xfb], resolution.operands);
        assert_eq!(None, resolution.target_address);
        assert_eq!("$0FFD             ".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_negative_edge() {
        let mut memory = Memory::new_with_ram();
        memory
            .write(
                0x0ffa,
                &[
                    0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0xd0, 0x80, 0x22, 0x00, 0x12, 0x0a,
                ],
            )
            .unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Relative(0x1000, [0x80]);
        assert_eq!("$0F82".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x80], resolution.operands);
        assert_eq!(None, resolution.target_address);
        assert_eq!("$0F82             ".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_relative_positive_edge() {
        let mut memory = Memory::new_with_ram();
        memory
            .write(
                0x0ffa,
                &[
                    0xfa, 0xfb, 0xfc, 0xfd, 0xfe, 0xff, 0xd0, 0x7f, 0x22, 0x00, 0x12, 0x0a,
                ],
            )
            .unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::Relative(0x1000, [0x7f]);
        assert_eq!("$1081".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x7f], resolution.operands);
        assert_eq!(None, resolution.target_address);
        assert_eq!("$1081             ".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_indirect() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x22]).unwrap();
        memory.write(0x0021, &[0x05, 0x80]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPageIndirect([0x21]);
        assert_eq!("($21)".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21], resolution.operands);
        assert_eq!(0x8005, resolution.target_address.unwrap());
        assert_eq!("($21)    (#0x8005)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_absolute_x_indexed_indirect() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x21, 0x20]).unwrap();
        memory.write(0x2025, &[0x05, 0x80]).unwrap();
        let mut registers = Registers::new(0x1000);
        let am = AddressingMode::AbsoluteXIndexedIndirect([0x21, 0x20]);
        assert_eq!("($2021,X)".to_owned(), format!("{}", am));

        registers.register_x = 0x04;
        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x21, 0x20], resolution.operands);
        assert_eq!(0x8005, resolution.target_address.unwrap());
        assert_eq!("($2021,X)(#0x8005)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_zero_page_relative() {
        let mut memory = Memory::new_with_ram();
        memory.write(0x1000, &[0xa5, 0x25, 0x20]).unwrap();
        memory.write(0x0025, &[0x05, 0x80]).unwrap();
        let registers = Registers::new(0x1000);
        let am = AddressingMode::ZeroPageRelative(0x1000, [0x25, 0x20]);
        assert_eq!("$25,$1022".to_owned(), format!("{}", am));

        let resolution: AddressingModeResolution = am.solve(0x1000, &memory, &registers).unwrap();
        assert_eq!(vec![0x25, 0x20], resolution.operands);
        assert_eq!(0x0025, resolution.target_address.unwrap());
        assert_eq!("$25,$1022(#0x0025)".to_owned(), format!("{}", resolution));
    }

    #[test]
    fn test_address_wraparound_zero_page_indirect_y_indexed() {
        let mut memory = Memory::new_with_ram();
        // Set up indirect pointer at zero page location 0x21 to point to 0xFFFE
        memory.write(0x0021, &[0xFE, 0xFF]).unwrap();
        // Set up some data at 0xFFFE and 0x0000 (after wrap)
        memory.write(0xFFFE, &[0x42]).unwrap();
        memory.write(0x0000, &[0x24]).unwrap();

        let mut registers = Registers::new(0x1000);
        registers.register_y = 2;  // This should cause 0xFFFE + 2 = 0x10000 to wrap to 0x0000

        let am = AddressingMode::ZeroPageIndirectYIndexed([0x21]);
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        assert_eq!(0x0000, resolution.target_address.unwrap()); // Should wrap to 0x0000
    }

    #[test]
    fn test_zero_page_pointer_wraparound() {
        let mut memory = Memory::new_with_ram();
        // Test the specific Stack Overflow case: LDA ($FF),Y
        // The pointer should read from $FF (LSB) and $00 (MSB) due to zero page wraparound
        memory.write(0x00FF, &[0x34]).unwrap(); // LSB at $FF
        memory.write(0x0000, &[0x12]).unwrap(); // MSB at $00 (wraps around in zero page)
        // So the 16-bit address is 0x1234

        let mut registers = Registers::new(0x1000);
        registers.register_y = 1;  // Add 1 to the base address

        let am = AddressingMode::ZeroPageIndirectYIndexed([0xFF]);
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        // Should read pointer from $FF and $00, giving 0x1234, then add Y=1 to get 0x1235
        assert_eq!(0x1235, resolution.target_address.unwrap());
    }

    #[test]
    fn test_zero_page_x_indexed_indirect_wraparound() {
        let mut memory = Memory::new_with_ram();
        // Test the Stack Overflow case: LDA ($FF,X) with X=0
        // The pointer should read from $FF (LSB) and $00 (MSB) due to zero page wraparound
        memory.write(0x00FF, &[0x56]).unwrap(); // LSB at $FF
        memory.write(0x0000, &[0x78]).unwrap(); // MSB at $00 (wraps around in zero page)
        // So the 16-bit address is 0x7856

        let mut registers = Registers::new(0x1000);
        registers.register_x = 0;  // X=0, so ($FF,X) = ($FF)

        let am = AddressingMode::ZeroPageXIndexedIndirect([0xFF]);
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        // Should read pointer from $FF and $00, giving 0x7856
        assert_eq!(0x7856, resolution.target_address.unwrap());
    }

    #[test]
    fn test_zero_page_x_indexed_indirect_x_wraparound() {
        let mut memory = Memory::new_with_ram();
        // Test X register causing wraparound: LDA ($FE,X) with X=2
        // The pointer calculation: $FE + 2 = $100, but should wrap to $00 in zero page
        // So we should read from $00 (LSB) and $01 (MSB) 
        memory.write(0x0000, &[0xAB]).unwrap(); // LSB at $00
        memory.write(0x0001, &[0xCD]).unwrap(); // MSB at $01
        // So the 16-bit address is 0xCDAB

        let mut registers = Registers::new(0x1000);
        registers.register_x = 2;  // X=2, so ($FE,X) = ($FE + 2) = ($100) wraps to ($00)

        let am = AddressingMode::ZeroPageXIndexedIndirect([0xFE]);
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        // Should read pointer from $00 and $01, giving 0xCDAB
        assert_eq!(0xCDAB, resolution.target_address.unwrap());
    }

    #[test]
    fn test_address_wraparound_absolute_x_indexed() {
        let memory = Memory::new_with_ram();
        let mut registers = Registers::new(0x1000);
        registers.register_x = 2;  // This should cause 0xFFFF + 2 = 0x10001 to wrap to 0x0001

        let am = AddressingMode::AbsoluteXIndexed([0xFF, 0xFF]); // 0xFFFF
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        assert_eq!(0x0001, resolution.target_address.unwrap()); // Should wrap to 0x0001
    }

    #[test]
    fn test_address_wraparound_absolute_y_indexed() {
        let memory = Memory::new_with_ram();
        let mut registers = Registers::new(0x1000);
        registers.register_y = 3;  // This should cause 0xFFFF + 3 = 0x10002 to wrap to 0x0002

        let am = AddressingMode::AbsoluteYIndexed([0xFF, 0xFF]); // 0xFFFF
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        assert_eq!(0x0002, resolution.target_address.unwrap()); // Should wrap to 0x0002
    }

    #[test]
    fn test_address_wraparound_absolute_x_indexed_indirect() {
        let mut memory = Memory::new_with_ram();
        // Test intermediate address wraparound: JMP ($FFFF,X) with X=2
        // Intermediate address: $FFFF + 2 = $10001, should wrap to $0001
        // Set up pointer at $0001 to point to final address $ABCD
        memory.write(0x0001, &[0xCD, 0xAB]).unwrap(); // Final address 0xABCD

        let mut registers = Registers::new(0x1000);
        registers.register_x = 2;  // X=2, so ($FFFF,X) = ($FFFF + 2) wraps to ($0001)

        let am = AddressingMode::AbsoluteXIndexedIndirect([0xFF, 0xFF]); // 0xFFFF
        let resolution = am.solve(0x1000, &memory, &registers).unwrap();

        // Should read pointer from wrapped address $0001, giving final address 0xABCD
        assert_eq!(0xABCD, resolution.target_address.unwrap());
    }

    #[test]
    fn test_page_boundary_crossing() {
        let mut registers = Registers::new(0x1000);
        let mut memory = Memory::new_with_ram();

        // Test AbsoluteXIndexed crossing page boundary
        registers.register_x = 0xFF;
        let am = AddressingMode::AbsoluteXIndexed([0x01, 0x20]); // Base addr: 0x2001
        assert!(am.needs_page_crossing_cycle(&registers, &memory)); // 0x2001 + 0xFF = 0x2100 (crosses page)

        registers.register_x = 0x01;
        assert!(!am.needs_page_crossing_cycle(&registers, &memory)); // 0x2001 + 0x01 = 0x2002 (same page)

        // Test AbsoluteYIndexed crossing page boundary
        registers.register_y = 0xFF;
        let am = AddressingMode::AbsoluteYIndexed([0x01, 0x20]); // Base addr: 0x2001
        assert!(am.needs_page_crossing_cycle(&registers, &memory)); // 0x2001 + 0xFF = 0x2100 (crosses page)

        registers.register_y = 0x01;
        assert!(!am.needs_page_crossing_cycle(&registers, &memory)); // 0x2001 + 0x01 = 0x2002 (same page)

        // Test ZeroPageIndirectYIndexed crossing page boundary
        registers.register_y = 0xFF;
        // Set up memory at zero page address 0x01 to contain 0x0201
        // This means base address is 0x0201, and with Y=0xFF will cross page (0x0201 + 0xFF = 0x0300)
        memory.write(0x01, &[0x01, 0x02]).unwrap();
        let am = AddressingMode::ZeroPageIndirectYIndexed([0x01]); 
        assert!(am.needs_page_crossing_cycle(&registers, &memory));

        registers.register_y = 0x01;
        // Set up memory at zero page address 0x01 to contain 0x0250
        // This means base address is 0x0250, and with Y=0x01 will not cross page (0x0250 + 0x01 = 0x0251)
        memory.write(0x01, &[0x50, 0x02]).unwrap();
        assert!(!am.needs_page_crossing_cycle(&registers, &memory));

        // Test Relative mode crossing page boundary
        // Branch from 0x20FD
        // Next instruction would be at 0x20FF (page 0x20)
        // Target = 0x20FF + 2 = 0x2101 (page 0x21)
        let am = AddressingMode::Relative(0x20FD, [0x02]);
        assert!(am.needs_page_crossing_cycle(&registers, &memory), 
            "Branch crossing forward to next page should need extra cycle");

        // Branch from 0x2001
        // Next instruction would be at 0x2003 (page 0x20)
        // Target = 0x2003 - 6 = 0x1FFD (page 0x1F)
        let am = AddressingMode::Relative(0x2001, [0xFA]); // -6 in two's complement
        assert!(am.needs_page_crossing_cycle(&registers, &memory),
            "Branch crossing backward to previous page should need extra cycle");

        // Branch within same page
        let am = AddressingMode::Relative(0x2050, [0x04]); // Small forward branch
        assert!(!am.needs_page_crossing_cycle(&registers, &memory),
            "Branch within same page should not need extra cycle");

        // Branch to immediately next instruction (BVC LABEL; LABEL:)
        let am = AddressingMode::Relative(0x2000, [0x00]);
        assert!(!am.needs_page_crossing_cycle(&registers, &memory),
            "Branch to next instruction should never need extra cycle");
    }
}
