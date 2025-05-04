use super::{Memory, Registers, AddressableIO};
use std::fmt;

#[derive(Debug)]
pub enum Source {
    Accumulator,
    RegisterX,
    RegisterY,
    RegisterS,
    RegisterSP,
    RegisterCP,
    Memory(usize),
}

impl Source {
    pub fn get_value(&self, registers: &Registers, memory: &Memory) -> usize {
        match *self {
            Source::Accumulator => registers.accumulator as usize,
            Source::RegisterX => registers.register_x as usize,
            Source::RegisterY => registers.register_y as usize,
            Source::RegisterSP => registers.get_status_register() as usize,
            Source::RegisterS => registers.stack_pointer as usize,
            Source::Memory(addr) => memory.read(addr, 1).unwrap()[0] as usize,
            Source::RegisterCP => registers.command_pointer as usize,
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Accumulator => write!(f, "A"),
            Source::RegisterX => write!(f, "X"),
            Source::RegisterY => write!(f, "Y"),
            Source::RegisterSP => write!(f, "SP"),
            Source::RegisterS => write!(f, "S"),
            Source::Memory(addr) => write!(f, "#0x{:04X}", addr),
            Source::RegisterCP => write!(f, "CP"),
        }
    }
}

#[derive(Debug)]
pub enum BooleanExpression {
    Equal(Source, usize),
    GreaterOrEqual(Source, usize),
    StrictlyGreater(Source, usize),
    LesserOrEqual(Source, usize),
    StrictlyLesser(Source, usize),
    Different(Source, usize),
    Value(bool),
    And(Box<BooleanExpression>, Box<BooleanExpression>),
    Or(Box<BooleanExpression>, Box<BooleanExpression>),
    MemorySequence(Source, Vec<u8>),
}

impl BooleanExpression {
    pub fn solve(&self, registers: &Registers, memory: &Memory) -> bool {
        match &*self {
            BooleanExpression::Equal(source, val) => source.get_value(registers, memory) == *val,
            BooleanExpression::GreaterOrEqual(source, val) => {
                source.get_value(registers, memory) >= *val
            }
            BooleanExpression::StrictlyGreater(source, val) => {
                source.get_value(registers, memory) > *val
            }
            BooleanExpression::LesserOrEqual(source, val) => {
                source.get_value(registers, memory) <= *val
            }
            BooleanExpression::StrictlyLesser(source, val) => {
                source.get_value(registers, memory) < *val
            }
            BooleanExpression::Different(source, val) => {
                source.get_value(registers, memory) != *val
            }
            BooleanExpression::Value(val) => *val,
            BooleanExpression::And(expr1, expr2) => {
                expr1.solve(registers, memory) && expr2.solve(registers, memory)
            }
            BooleanExpression::Or(expr1, expr2) => {
                expr1.solve(registers, memory) || expr2.solve(registers, memory)
            }
            BooleanExpression::MemorySequence(source, expected_bytes) => {
                if let Source::Memory(addr) = source {
                    if let Ok(actual_bytes) = memory.read(*addr, expected_bytes.len()) {
                        actual_bytes == expected_bytes
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

impl fmt::Display for BooleanExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BooleanExpression::Equal(source, val) => write!(f, "{} = 0x{:X}", source, val),
            BooleanExpression::GreaterOrEqual(source, val) => {
                write!(f, "{} ≥ 0x{:X}", source, val)
            }
            BooleanExpression::StrictlyGreater(source, val) => {
                write!(f, "{} > 0x{:X}", source, val)
            }
            BooleanExpression::LesserOrEqual(source, val) => {
                write!(f, "{} ≤ 0x{:X}", source, val)
            }
            BooleanExpression::StrictlyLesser(source, val) => {
                write!(f, "{} < 0x{:X}", source, val)
            }
            BooleanExpression::Different(source, val) => {
                write!(f, "{} ≠ 0x{:X}", source, val)
            }
            BooleanExpression::Value(val) => {
                write!(f, "{}", if *val { "true" } else { "false" })
            }
            BooleanExpression::And(expr1, expr2) => {
                write!(f, "{} AND {}", expr1, expr2)
            }
            BooleanExpression::Or(expr1, expr2) => {
                write!(f, "({} OR {})", expr1, expr2)
            }
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

