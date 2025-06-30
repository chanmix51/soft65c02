use std::fmt;

mod error;
mod memory_stack;
mod ram;
mod rom;

pub use error::MemoryError;
pub use memory_stack::MemoryStack;
pub use ram::RAM;
pub use rom::ROM;

pub const MEMMAX: usize = 65535;

pub fn little_endian(bytes: Vec<u8>) -> usize {
    let mut addr: usize = 0;

    for byte in bytes.iter().rev() {
        addr = addr << 8 | (*byte as usize);
    }

    addr
}

/*
 * AddressableIO
 * this trait defines the interface for all memory systems
 */
pub trait AddressableIO {
    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError>;
    fn write(&mut self, location: usize, data: &[u8]) -> Result<(), MemoryError>;
    fn get_size(&self) -> usize;
}

/*
 * DisplayBackend
 * This trait defines the interface for display/graphics backends
 * Graphics backends implement this and can be used as memory-mapped I/O
 */
pub trait DisplayBackend: AddressableIO + Send {
    /// Get the display dimensions
    fn get_dimensions(&self) -> (usize, usize);
    
    /// Check if the display window is still open/active
    fn is_active(&self) -> bool;
    
    /// Get keyboard input events if available
    fn get_input_events(&mut self) -> Vec<u32>;
}

pub trait DebugIO: AddressableIO {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        todo!()
        /*
               let mut line = String::new();
               let address = 0;
               let end = self.get_size();
               let bytes = self.read(0, end).unwrap();

               while address < end {
                   if address % 16 == 0 {
                       write!(f, "{}", line).unwrap();
                       line = format!("#{:04X}: ", address);
                   } else if address % 8 == 0 {
                       line = format!("{} ", line);
                   }

                   line = format!("{} {:02x}", line, bytes[address]);
               }

               write!(f, "{}", line)
        */
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
