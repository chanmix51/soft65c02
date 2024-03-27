use std::error;
use std::fmt;

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub enum MemoryError {
    ReadOverflow(usize, usize), // read len, address
    WriteOverflow(usize, usize), // write len, address
    Other(usize, &'static str),        // address, error message
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MemoryError::ReadOverflow(read_len, addr) => write!(
                f,
                "Could not READ {} bytes at address 0x{:04X}",
                read_len, addr
            ),
            MemoryError::WriteOverflow(read_len, addr) => write!(
                f,
                "Could not WRITE {} bytes at address 0x{:04X}",
                read_len, addr
            ),
            MemoryError::Other(addr, err_msg) => {
                write!(f, "Memory error @{:04X} with message: {}", addr, err_msg)
            }
        }
    }
}

impl error::Error for MemoryError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}
