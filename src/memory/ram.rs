use super::*;

pub struct RAM {
    ram: Box<[u8; MEMMAX + 1]>,
}

impl RAM {
    pub fn new() -> RAM {
        RAM { ram: Box::new([0x00; MEMMAX + 1]) }
    }
}

impl AddressableIO for RAM {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if self.ram.len() >= addr + len {
            Ok(self.ram[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr, self.ram.len()))
        }
    }

    fn write(&mut self, location: usize, data: Vec<u8>) -> Result<(), MemoryError> {
        if location + data.len() > self.ram.len() {
            Err(MemoryError::WriteOverflow(data.len(), location, self.ram.len()))
        } else {
            for offset in 0..data.len() {
                self.ram[usize::from(location) + offset] = data[offset];
            }

            Ok(())
        }
    }

    fn get_size(&self) -> usize {
        self.ram.len()
    }
}

impl DebugIO for RAM {
}
