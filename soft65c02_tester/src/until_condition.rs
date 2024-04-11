use anyhow::anyhow;
use soft65c02_lib::{AddressableIO, Memory, Registers};
use std::fmt;

use crate::AppResult;

#[derive(Debug)]
pub enum RegisterSource {
    Accumulator,
    RegisterX,
    RegisterY,
    Status,
    StackPointer,
    CommandPointer,
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
}

impl BooleanExpression {
    pub fn solve(&self, registers: &Registers, memory: &Memory) -> bool {
        match self {
            BooleanExpression::Equal(left, right) => {
                left.get_value(registers, memory) == right.get_value(registers, memory)
            }
            BooleanExpression::GreaterOrEqual(left, right) => {
                left.get_value(registers, memory) >= right.get_value(registers, memory)
            }
            BooleanExpression::StrictlyGreater(left, right) => {
                left.get_value(registers, memory) > right.get_value(registers, memory)
            }
            BooleanExpression::LesserOrEqual(left, right) => {
                left.get_value(registers, memory) <= right.get_value(registers, memory)
            }
            BooleanExpression::StrictlyLesser(left, right) => {
                left.get_value(registers, memory) > right.get_value(registers, memory)
            }
            BooleanExpression::Different(left, right) => {
                left.get_value(registers, memory) != right.get_value(registers, memory)
            }
            BooleanExpression::Value(val) => *val,
            BooleanExpression::And(expr1, expr2) => {
                expr1.solve(registers, memory) && expr2.solve(registers, memory)
            }
            BooleanExpression::Or(expr1, expr2) => {
                expr1.solve(registers, memory) || expr2.solve(registers, memory)
            }
        }
    }
}

impl fmt::Display for BooleanExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BooleanExpression::Equal(left, right) => write!(f, "{left} = {right}"),
            BooleanExpression::GreaterOrEqual(left, right) => {
                write!(f, "{left} ≥ {right}")
            }
            BooleanExpression::StrictlyGreater(left, right) => {
                write!(f, "{left} > {right}")
            }
            BooleanExpression::LesserOrEqual(left, right) => {
                write!(f, "{left} ≤ {right}")
            }
            BooleanExpression::StrictlyLesser(left, right) => {
                write!(f, "{left} < {right}")
            }
            BooleanExpression::Different(left, right) => {
                write!(f, "{left} ≠ {right}")
            }
            BooleanExpression::Value(val) => {
                write!(f, "{}", if *val { "true" } else { "false" })
            }
            BooleanExpression::And(expr1, expr2) => {
                write!(f, "{expr1} AND {expr2}")
            }
            BooleanExpression::Or(expr1, expr2) => {
                write!(f, "({expr1} OR {expr2})")
            }
        }
    }
}
