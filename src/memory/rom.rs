use super::*;
const ROM_SIZE:usize = 16*1024;

pub struct ROM {
    rom: Box<[u8; ROM_SIZE]>,
}

impl ROM {
    pub fn new(data: [u8; ROM_SIZE]) -> ROM {
        ROM { rom: Box::new(data) }
    }
}

impl AddressableIO for ROM {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if self.rom.len() >= addr + len {
            Ok(self.rom[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr, self.rom.len()))
        }
    }

    fn write(&mut self, location: usize, _data: Vec<u8>) -> Result<(), MemoryError> {
        Err(MemoryError::Other(
            location,
            "trying to write in a read-only memory",
        ))
    }

    fn get_size(&self) -> usize {
        self.rom.len()
    }
}

impl DebugIO for ROM {}
