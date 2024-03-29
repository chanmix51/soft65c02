use super::*;

pub struct RAM {
    ram: Box<[u8; MEMMAX + 1]>,
}

impl Default for RAM {
    fn default() -> Self {
        Self {
            ram: Box::new([0x00; MEMMAX + 1]),
        }
    }
}

impl AddressableIO for RAM {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if self.ram.len() >= addr + len {
            Ok(self.ram[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr))
        }
    }

    fn write(&mut self, location: usize, data: &[u8]) -> Result<(), MemoryError> {
        if location + data.len() > self.ram.len() {
            Err(MemoryError::WriteOverflow(data.len(), location))
        } else {
            self.ram
                .iter_mut()
                .enumerate()
                .skip(location)
                .for_each(|(index, value)| *value = data[index]);
            // for offset in 0..data.len() {
            //     self.ram[usize::from(location) + offset] = data[offset];
            // }

            Ok(())
        }
    }

    fn get_size(&self) -> usize {
        self.ram.len()
    }
}

impl DebugIO for RAM {}
