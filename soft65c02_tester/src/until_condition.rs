use soft65c02_lib::{AddressableIO, Memory, Registers};
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
    Value(usize),
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
            Source::RegisterCP => registers.command_pointer,
            Source::Value(data) => data,
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
            Source::Memory(addr) => write!(f, "#0x{addr:04X}"),
            Source::RegisterCP => write!(f, "CP"),
            Source::Value(data) => write!(f, "0x{data:02X}"),
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
