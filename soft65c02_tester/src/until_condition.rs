use anyhow::anyhow;
use soft65c02_lib::{AddressableIO, Memory, Registers};
use std::fmt::{self};

use crate::AppResult;

#[derive(Debug)]
pub enum RegisterSource {
    Accumulator,
    RegisterX,
    RegisterY,
    Status,
    StackPointer,
    CommandPointer,
    CycleCount,
}

impl RegisterSource {
    pub fn get_value(&self, registers: &Registers) -> usize {
        match self {
            Self::Accumulator => registers.accumulator as usize,
            Self::RegisterX => registers.register_x as usize,
            Self::RegisterY => registers.register_y as usize,
            Self::Status => registers.get_status_register() as usize,
            Self::StackPointer => registers.stack_pointer as usize,
            Self::CommandPointer => registers.command_pointer,
            Self::CycleCount => registers.cycle_count as usize,
        }
    }
}

impl fmt::Display for RegisterSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Accumulator => write!(f, "A"),
            Self::RegisterX => write!(f, "X"),
            Self::RegisterY => write!(f, "Y"),
            Self::Status => write!(f, "S"),
            Self::StackPointer => write!(f, "SP"),
            Self::CommandPointer => write!(f, "CP"),
            Self::CycleCount => write!(f, "cycle_count"),
        }
    }
}

#[derive(Debug)]
pub struct Assignment {
    pub source: Source,
    pub destination: RegisterSource,
}

impl Assignment {
    pub fn new(source: Source, destination: RegisterSource) -> Self {
        Self {
            source,
            destination,
        }
    }

    pub fn execute(&self, registers: &mut Registers, memory: &Memory) -> AppResult<Vec<String>> {
        let output = match self.destination {
            RegisterSource::Accumulator => {
                let val = Self::to_u8(self.source.get_value(registers, memory))?;
                registers.accumulator = val;

                format!("register A set to 0x{val:02x}")
            }
            RegisterSource::RegisterX => {
                let val = Self::to_u8(self.source.get_value(registers, memory))?;
                registers.register_x = val;

                format!("register X set to 0x{val:02x}")
            }
            RegisterSource::RegisterY => {
                let val = Self::to_u8(self.source.get_value(registers, memory))?;
                registers.register_y = val;

                format!("register Y set to 0x{val:02x}")
            }
            RegisterSource::Status => {
                let val = Self::to_u8(self.source.get_value(registers, memory))?;
                registers.set_status_register(val);

                format!("register S set to 0x{val:02x}")
            }
            RegisterSource::StackPointer => {
                let val = self.source.get_value(registers, memory);
                registers.stack_pointer = Self::to_u8(val)?;

                format!("register SP set to 0x{val:02x}")
            }
            RegisterSource::CommandPointer => {
                let val = self.source.get_value(registers, memory);
                registers.command_pointer = val;

                format!("register CP set to #0x{val:04x}")
            }
            RegisterSource::CycleCount => {
                let val = self.source.get_value(registers, memory);
                registers.cycle_count = val as u64;

                format!("cycle_count set to {val}")
            }
        };

        Ok(vec![output])
    }

    fn to_u8(val: usize) -> AppResult<u8> {
        if val > 255 {
            Err(anyhow!("Value {val} cannot fit in 8 bits destination."))
        } else {
            let bytes = val.to_le_bytes();

            Ok(bytes[0])
        }
    }
}

#[cfg(test)]
mod assignment_tests {
    use super::*;

    #[test]
    fn test_to_u8() {
        assert_eq!(0, Assignment::to_u8(0).expect("0 is a valid 8 bits number"));
        assert_eq!(
            255,
            Assignment::to_u8(255).expect("255 is a valid 8 bits number")
        );
        Assignment::to_u8(256).expect_err("9 bits usize can not fit in 8 bits");
    }
}

#[derive(Debug)]
pub enum Source {
    Register(RegisterSource),
    Memory(usize),
    Value(usize),
}

impl Source {
    pub fn get_value(&self, registers: &Registers, memory: &Memory) -> usize {
        match self {
            Self::Register(register_source) => register_source.get_value(registers),
            Self::Memory(addr) => memory.read(*addr, 1).unwrap()[0] as usize,
            Self::Value(data) => *data,
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Register(register_source) => write!(f, "{register_source}"),
            Self::Memory(addr) => write!(f, "#0x{addr:04X}"),
            Self::Value(data) => write!(f, "0x{data:02X}"),
        }
    }
}

#[derive(Debug)]
pub enum BooleanExpression {
    Equal(Source, Source),
    GreaterOrEqual(Source, Source),
    StrictlyGreater(Source, Source),
    LesserOrEqual(Source, Source),
    StrictlyLesser(Source, Source),
    Different(Source, Source),
    Value(bool),
    And(Box<BooleanExpression>, Box<BooleanExpression>),
    Or(Box<BooleanExpression>, Box<BooleanExpression>),
    MemorySequence(Source, Vec<u8>),  // For comparing memory contents against a sequence of bytes
}

impl BooleanExpression {
    /// Solve the boolean expression with the given registers and memory.
    /// If the expression is true, None is returned. Otherwise, the failure message is returned.
    pub fn solve(&self, registers: &Registers, memory: &Memory) -> Option<String> {
        match self {
            BooleanExpression::Equal(left, right) => {
                let left_value = left.get_value(registers, memory);
                let right_value = right.get_value(registers, memory);

                if left_value != right_value {
                    Some(format!(
                        "({self})  0x{:02x} is not equal to 0x{:02x}",
                        left_value, right_value
                    ))
                } else {
                    None
                }
            }
            BooleanExpression::GreaterOrEqual(left, right) => {
                let left_value = left.get_value(registers, memory);
                let right_value = right.get_value(registers, memory);

                if left_value < right_value {
                    Some(format!(
                        "({self}) 0x{:02x} is not greater than or equal to 0x{:02x}",
                        left_value, right_value
                    ))
                } else {
                    None
                }
            }
            BooleanExpression::StrictlyGreater(left, right) => {
                let left_value = left.get_value(registers, memory);
                let right_value = right.get_value(registers, memory);

                if left_value <= right_value {
                    Some(format!(
                        "({self}) 0x{:02x} is not strictly greater than 0x{:02x}",
                        left_value, right_value
                    ))
                } else {
                    None
                }
            }
            BooleanExpression::LesserOrEqual(left, right) => {
                let left_value = left.get_value(registers, memory);
                let right_value = right.get_value(registers, memory);

                if left_value > right_value {
                    Some(format!(
                        "({self}) 0x{:02x} is not lesser than or equal to 0x{:02x}",
                        left_value, right_value
                    ))
                } else {
                    None
                }
            }
            BooleanExpression::StrictlyLesser(left, right) => {
                let left_value = left.get_value(registers, memory);
                let right_value = right.get_value(registers, memory);

                if left_value >= right_value {
                    Some(format!(
                        "({self}) 0x{:02x} is not strictly lesser than 0x{:02x}",
                        left_value, right_value
                    ))
                } else {
                    None
                }
            }
            BooleanExpression::Different(left, right) => {
                let left_value = left.get_value(registers, memory);
                let right_value = right.get_value(registers, memory);

                if left_value == right_value {
                    Some(format!(
                        "({self}) 0x{:02x} is equal to 0x{:02x}",
                        left_value, right_value
                    ))
                } else {
                    None
                }
            }
            BooleanExpression::Value(val) => {
                if !val {
                    Some(format!("({self}) is false"))
                } else {
                    None
                }
            }
            BooleanExpression::And(expr1, expr2) => {
                if let Some(msg) = expr1.solve(registers, memory) {
                    Some(msg)
                } else {
                    expr2.solve(registers, memory)
                }
            }
            BooleanExpression::Or(expr1, expr2) => {
                if let Some(msg1) = expr1.solve(registers, memory) {
                    if let Some(msg2) = expr2.solve(registers, memory) {
                        Some(format!("({self}) both conditions failed:\n  {msg1}\n  {msg2}"))
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            BooleanExpression::MemorySequence(source, expected_bytes) => {
                if let Source::Memory(addr) = source {
                    if let Ok(actual_bytes) = memory.read(*addr, expected_bytes.len()) {
                        if &actual_bytes == expected_bytes {
                            None
                        } else {
                            Some(format!(
                                "({self}) Memory at #0x{:04x} contains {:02x?} instead of expected {:02x?}",
                                addr, actual_bytes, expected_bytes
                            ))
                        }
                    } else {
                        Some(format!(
                            "({self}) Failed to read {} bytes from memory at #0x{:04x}",
                            expected_bytes.len(), addr
                        ))
                    }
                } else {
                    Some(format!(
                        "({self}) Memory sequence comparison requires a memory address source, got {}",
                        source
                    ))
                }
            }
        }
    }
}

impl fmt::Display for BooleanExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BooleanExpression::Equal(source, val) => write!(f, "{} = {}", source, val),
            BooleanExpression::GreaterOrEqual(source, val) => write!(f, "{} >= {}", source, val),
            BooleanExpression::StrictlyGreater(source, val) => write!(f, "{} > {}", source, val),
            BooleanExpression::LesserOrEqual(source, val) => write!(f, "{} <= {}", source, val),
            BooleanExpression::StrictlyLesser(source, val) => write!(f, "{} < {}", source, val),
            BooleanExpression::Different(source, val) => write!(f, "{} != {}", source, val),
            BooleanExpression::Value(val) => write!(f, "{}", if *val { "true" } else { "false" }),
            BooleanExpression::And(expr1, expr2) => write!(f, "{} AND {}", expr1, expr2),
            BooleanExpression::Or(expr1, expr2) => write!(f, "({} OR {})", expr1, expr2),
            BooleanExpression::MemorySequence(source, bytes) => {
                write!(f, "{} ~ 0x({})", source, 
                    bytes.iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests_boolean_expression {
    use super::*;
    use soft65c02_lib::Memory;

    #[test]
    fn test_memory_sequence_comparison() {
        let mut memory = Memory::new_with_ram();
        let registers = Registers::new(0);
        
        // Write test sequence to memory
        memory.write(0x8000, &[0x01, 0xa2, 0xf3]).unwrap();
        
        // Test matching sequence
        let expr = BooleanExpression::MemorySequence(
            Source::Memory(0x8000),
            vec![0x01, 0xa2, 0xf3]
        );
        assert!(expr.solve(&registers, &memory).is_none());
        
        // Test non-matching sequence
        let expr = BooleanExpression::MemorySequence(
            Source::Memory(0x8000),
            vec![0x01, 0xa2, 0xf4]  // Different last byte
        );
        assert!(expr.solve(&registers, &memory).is_some());
        
        // Test with non-memory source
        let expr = BooleanExpression::MemorySequence(
            Source::Register(RegisterSource::Accumulator),
            vec![0x01, 0x02]
        );
        assert!(expr.solve(&registers, &memory).is_some());
        
        // Test with out-of-bounds memory access
        let expr = BooleanExpression::MemorySequence(
            Source::Memory(0xffff),
            vec![0x01, 0x02]  // Trying to read past end of memory
        );
        assert!(expr.solve(&registers, &memory).is_some());
    }

    #[test]
    fn test_memory_sequence_display() {
        let expr = BooleanExpression::MemorySequence(
            Source::Memory(0x8000),
            vec![0x01, 0xa2, 0xf3]
        );
        assert_eq!(
            format!("{}", expr),
            "#0x8000 ~ 0x(01,a2,f3)"
        );
    }
}
