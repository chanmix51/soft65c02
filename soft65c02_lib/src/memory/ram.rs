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
            for (offset, value) in data.iter().enumerate() {
                self.ram[location + offset] = *value;
            }
            Ok(())
        }
    }

    fn get_size(&self) -> usize {
        self.ram.len()
    }
}

impl DebugIO for RAM {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_write_ram() {
        let mut memory = RAM::default();
        memory.write(1000, &[0x01, 0x02, 0x03]).unwrap();

        assert_eq!(1, memory.ram[1000]);
        assert_eq!(2, memory.ram[1001]);
        assert_eq!(3, memory.ram[1002]);
        assert_eq!(0, memory.ram[1003]);
    }

    #[test]
    fn check_read_ram() {
        let mut memory = RAM::default();
        memory.ram[1000] = 0xff;

        assert_eq!(vec![0x00, 0xff, 0x00], memory.read(999, 3).unwrap());
    }
}
