use std::error;
use std::fmt;

pub const MEMMAX:usize = 65535;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum MemoryError {
    ReadOverflow(usize, usize, usize),
    WriteOverflow(usize, usize, usize),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MemoryError::ReadOverflow(read_len, addr, total_len) =>
                write!(f, "Could not READ {} bytes at address 0x{:04X}, address max is 0x{:04X}.", read_len, addr, total_len),
            MemoryError::WriteOverflow(read_len, addr, total_len) =>
                write!(f, "Could not WRITE {} bytes at address 0x{:04X}, address max is 0x{:04X}.", read_len, addr, total_len),
        }
    }
}

impl error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub fn little_endian(bytes: Vec<u8>) -> usize {
    let mut bytes = bytes.clone();
    bytes.reverse();
    let mut addr:usize = 0;

    for byte in bytes {
        addr = addr << 8 | (byte as usize);
    }

    addr
}

pub struct RAM {
    ram: Box<[u8; MEMMAX + 1]>,
}

impl RAM {
    pub fn new() -> RAM {
        RAM { ram: Box::new([0x00; MEMMAX + 1]) }
    }

    pub fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if self.ram.len() >= addr + len {
            Ok(self.ram[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr, self.ram.len()))
        }
    }

    pub fn write(&mut self, location: usize, data: Vec<u8>) -> Result<(), MemoryError> {
        if location + data.len() > self.ram.len() {
            Err(MemoryError::WriteOverflow(data.len(), location, self.ram.len()))
        } else {
            for offset in 0..data.len() {
                self.ram[usize::from(location) + offset] = data[offset];
            }

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_little_endian() {
        assert_eq!(0x1234, little_endian(vec![0x34, 0x12]));
        assert_eq!(0x0011, little_endian(vec![0x11, 0x00]));
        assert_eq!(0x1100, little_endian(vec![0x00, 0x11]));
    }
}
