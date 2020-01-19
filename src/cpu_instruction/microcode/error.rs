use crate::addressing_mode;
use crate::memory;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum MicrocodeError {
    // ↓ when an overflow problem occures during the microcode operation
    MemoryOverflow(memory::MemoryError),
    Resolution(addressing_mode::ResolutionError),
    Runtime,
}

pub type Result<T> = std::result::Result<T, MicrocodeError>;

impl fmt::Display for MicrocodeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MicrocodeError::MemoryOverflow(e) => {
                write!(f, "memory overflow during microcode operation: {}", e)
            }
            MicrocodeError::Resolution(e) => {
                write!(f, "resolution error caught in microcode operation: {}", e)
            }
            MicrocodeError::Runtime => {
                write!(f, "runtime error while executing microcode operation")
            }
        }
    }
}

impl error::Error for MicrocodeError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl std::convert::From<addressing_mode::ResolutionError> for MicrocodeError {
    fn from(err: addressing_mode::ResolutionError) -> MicrocodeError {
        MicrocodeError::Resolution(err)
    }
}

impl std::convert::From<memory::MemoryError> for MicrocodeError {
    fn from(err: memory::MemoryError) -> MicrocodeError {
        MicrocodeError::MemoryOverflow(err)
    }
}
