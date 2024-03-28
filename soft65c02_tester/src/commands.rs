use crate::until_condition::BooleanExpression;
#[derive(Debug)]
pub enum CliCommand {
    Help(HelpCommand),
    None,
}

#[derive(Debug)]
pub enum HelpCommand {
    Global,
    Registers,
    Run,
}

#[derive(Debug)]
pub struct RunCommand {
    pub stop_condition: BooleanExpression,
    pub start_address: Option<usize>,
}
