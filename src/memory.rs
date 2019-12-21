const MEMMAX:usize = 65535;

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

    pub fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, String> {
        if self.ram.len() >= addr + len {
            Ok(self.ram[addr..addr + len].to_vec())
        } else {
            Err(format!("Could not read {} bytes at address 0x{:04X}, address max is 0x{:04X}.", len, addr, self.ram.len()))
        }
    }

    pub fn write(&mut self, location: usize, data: Vec<u8>) -> Result<(), String> {
        if location + data.len() > self.ram.len() {
            return Err(format!("Could not write {} bytes from 0x{:04X} cause it is exceeds max memory address 0x{:04X}.", data.len(), location, self.ram.len()));
        }

        for offset in 0..data.len() {
            self.ram[usize::from(location) + offset] = data[offset];
        }

        Ok(())
    }
}

