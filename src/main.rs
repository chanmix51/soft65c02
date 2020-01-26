extern crate ansi_term;

use ansi_term::Colour;

extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::error::Error as PestError;
use pest::iterators::{Pair, Pairs};
use pest::{Parser, RuleType};

extern crate rustyline;

use rustyline::error::ReadlineError;
use rustyline::{Context, Editor};
use rustyline::Result as RustyResult;

use soft65c02::{AddressableIO, LogLine, Memory, Registers};

use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::{thread, time};

#[derive(Parser)]
#[grammar = "cli.pest"]
pub struct BEParser;

#[derive(Debug)]
pub enum Source8 {
    Accumulator,
    RegisterX,
    RegisterY,
    RegisterS,
    RegisterSP,
    Memory(usize),
}

impl Source8 {
    pub fn get_value(&self, registers: &Registers, memory: &Vec<u8>) -> u8 {
        match *self {
            Source8::Accumulator => registers.accumulator,
            Source8::RegisterX => registers.register_x,
            Source8::RegisterY => registers.register_y,
            Source8::RegisterSP => registers.get_status_register(),
            Source8::RegisterS => registers.stack_pointer,
            Source8::Memory(addr) => memory[addr],
        }
    }
}

#[derive(Debug)]
pub enum BooleanExpression {
    Equal(Source8, u8),
    GreaterOrEqual(Source8, u8),
    StrictlyGreater(Source8, u8),
    LesserOrEqual(Source8, u8),
    StrictlyLesser(Source8, u8),
    Different(Source8, u8),
    Value(bool),
}

impl BooleanExpression {
    pub fn solve(&self, registers: &Registers, memory: &Vec<u8>) -> bool {
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
        }
    }
}

fn display_error<T: RuleType>(err: PestError<T>) {
    let (mark_str, msg) = match err.location {
        pest::error::InputLocation::Pos(x)  => {
            let mut pos_str = String::new();
            for _ in 0..x { pos_str.push(' '); }
            pos_str.push('↑');

            (pos_str, format!("at position {}", x))
        },
        pest::error::InputLocation::Span((a, b)) => {
            let mut pos_str = String::new();
            for _ in 0..a { pos_str.push(' '); }
            pos_str.push('↑');
            for _ in a..b { pos_str.push(' '); }
            pos_str.push('↑');
            (pos_str, format!("somewhere between position {} and {}", a, b))
        },
    };
    println!("   {}\n{} {}",
        mark_str,
        format!("{}", Colour::Red.paint("Syntax error")),
        msg
    );
    match err.variant {
        pest::error::ErrorVariant::ParsingError {positives, negatives: _} => {
            println!("{}", Colour::Fixed(240).paint(format!("hint: expected {:?}", positives)));
        },
        pest::error::ErrorVariant::CustomError { message } => {
            println!("{}", Colour::Fixed(240).paint(format!("message: {}", message)));
        },
    };

}

fn main() {
    let mut registers = Registers::new(0x0000);
    let mut memory = Memory::new_with_ram();

    println!("{}", Colour::Green.paint("Welcome in Soft-65C02 version 1.0.0-alpha1"));
    let prompt = format!("{}", Colour::Fixed(148).bold().paint(">> "));
    let mut rl = Editor::<CommandLineCompleter>::new();
    rl.set_helper(Some(CommandLineCompleter {}));
    loop {
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                if line.len() == 0 {
                    continue;
                }
                rl.add_history_entry(line.as_str());
                match BEParser::parse(Rule::sentence, line.as_str()) {
                    Ok(mut pairs)   => {
                        let response = parse_instruction(pairs.next().unwrap().into_inner(), &mut registers, &mut memory);
                    },
                    Err(parse_err)    => {
                        display_error(parse_err);
                    },
                };
            },
            Err(ReadlineError::Eof) => {
                println!("Quit!");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

pub fn parse_instruction(mut nodes: Pairs<Rule>, registers: &mut Registers, memory: &mut Memory) {
    let node = nodes.next().unwrap();
    match node.as_rule() {
        Rule::registers_instruction => exec_register_instruction(node.into_inner(), registers),
        Rule::memory_instruction    => exec_memory_instruction(node.into_inner(), memory),
        Rule::run_instruction       => println!("Run instruction"),
        Rule::help_instruction      => help(node.into_inner()),
        Rule::disassemble_instruction => println!("Disassemble instruction"),
        _   => {}, 
    };
}

fn exec_memory_instruction(mut nodes: Pairs<Rule>, memory: &mut Memory) {
    let node = nodes.next().unwrap();
    match node.as_rule() {
        Rule::memory_show   => {
            let mut subnodes = node.into_inner();
            let addr = parse_memory(subnodes.next().unwrap().as_str()[3..].to_owned());
            let len:usize = subnodes.next().unwrap().as_str().parse::<usize>().unwrap();
            for line in soft65c02::mem_dump(addr, len, memory).iter() {
                println!("{}", line);
            }
            
        },
        _   => { println!("{:?}", node); },
    }
}

fn exec_register_instruction(mut nodes: Pairs<Rule>, registers: &mut Registers) {
    let node = nodes.next().unwrap();
    match node.as_rule() {
        Rule::registers_show    => {
            println!("{:?}", registers);
        },
        Rule::registers_flush   => {
            registers.flush();
            println!("Registers flushed!");
        },
        _   => { println!("{:?}", node); },
    };
}

fn help(mut nodes: Pairs<Rule>) {
    if let Some(node) = nodes.next() {
        match node.as_rule() {
            Rule::help_registers => {
                println!("{}", Colour::Green.paint("Registers commands:"));
                println!("");
                println!("  registers show");
                println!("          Dump the content of the CPU registers.");
                println!("");
                println!("  registers flush");
                println!("          Reset the content of the CPU registers.");
            },
            Rule::help_memory   => {
                println!("{}", Colour::Green.paint("Memory commands:"));
                println!("  memory show ADDRESS LENGTH");
                println!("          Show the content of the memory starting from ADDRESS.");
                println!("          Example: {}", Colour::Fixed(240).paint("memory show #0x1234 100"));
                println!("");
                println!("   memory load ADDRESS \"filename.ext\" ");
                println!("          Load a binary file at the selected address in memory.");
                println!("          The content of the file is copied in the memory, so the memory has to");
                println!("          be writable.");
                println!("          Example: {}", Colour::Fixed(240).paint("memory load #0x1C00 \"program.bin\""));
            },
            Rule::help_run  => {
                println!("{}", Colour::Green.paint("Execution commands:"));
                println!("   run [ADDRESS] [until BOOLEAN_CONDITION]");
                println!("          Launch execution of the program.");
                println!("          Without further information, the execution goes on forever.");
                println!("          There is one condition for the execution to stop: the Command Pointer");
                println!("          to be at the exact same address before after an operand is executed.");
                println!("          This is the case for the STP (stop) instruction but also after");
                println!("          infinite loops like BRA -2 or a JMP at the exact same address.");
                println!("          It is possible to give extra conditions to break the execution of the");
                println!("          program, by example if it is desirable to execute step by step at a");
                println!("          certain point by using the \"until\" keyword");
                println!("");
                println!("{}", Colour::White.bold().paint("Examples:"));
                println!("  {}", Colour::Fixed(240).paint("run"));
                println!("          Launch execution starting at the actual CP register position.");
                println!("");
                println!("  {}", Colour::Fixed(240).paint("run 0x1C00"));
                println!("          Set the CP register at 0x1C00 and launch execution.");
                println!("");
                println!("{}", Colour::Green.paint("Boolean conditions"));
                println!("  It is possible to stop the current execution process to check the state of");
                println!("  the memory or CPU at this point. The execution is kept running until the");
                println!("  given condition is evaluated to be true.");
                println!("  It is possible to give any combination of >, <, <= >= or != from registers");
                println!("  or from a memory location.");
                println!("");
                println!("{}", Colour::White.bold().paint("Examples:"));
                println!("  {}", Colour::Fixed(240).paint("run until false"));
                println!("    The instruction is executed, this is step by step mode.");
                println!("");
                println!("  {}", Colour::Fixed(240).paint("run until A = 0x12"));
                println!("    The execution is launched until the A registers equals 0x12.");
                println!("");
                println!("  {}", Colour::Fixed(240).paint("run until 0x0200 > 0"));
                println!("    The execution is launched until the given memory address at is greater");
                println!("    than 0.");
                println!("");
                println!("  {}", Colour::Fixed(240).paint("run until S > 0x7f"));
                println!("    The execution is launched until the Negative flag of the status register is");
                println!("    set.");
            },
            Rule::help_disassemble => {
                println!("{}", Colour::Green.paint("Registers command:"));
                println!("");
                println!("  disassemble ADDR LENGTH");
                println!("         Disassemble start from address for the next \"operations\" instructions.");
                println!("          Example: {}", Colour::Fixed(240).paint("disassemble 0x1C00 100"));
                println!("          Disassemble 100 opcodes starting from address 0x1C00.");
            },
            _   => { },
        };
    } else {
        println!("{}", Colour::Green.paint("Available commands:"));
        println!("{}", Colour::White.bold().paint("Registers"));
        println!("  registers show");
        println!("          Dump the content of the CPU registers.");
        println!("  registers flush");
        println!("          Reset the content of the CPU registers.");
        println!("{}", Colour::White.bold().paint("Memory"));
        println!("   memory show ADDRESS LENGTH");
        println!("          Show the content of the memory starting from ADDRESS.");
        println!("   memory load ADDRESS \"filename.ext\" ");
        println!("          Load a binary file at the selected address in memory.");
        println!("{}", Colour::White.bold().paint("Execution"));
        println!("   run [ADDRESS] [until BOOLEAN_CONDITION]");
        println!("          Launch execution of the program.");
        println!("{}", Colour::White.bold().paint("Disassembler"));
        println!("   disassemble ADDRESS OPERATIONS");
        println!("         Disassemble start from address for the next \"operations\" instructions.");
        println!("{}", Colour::White.bold().paint("Help"));
        println!("   help [TOPIC]");
        println!("         Display informations about commands.");
    };
}

pub fn parse_boolex(mut nodes: Pairs<Rule>) -> BooleanExpression {
    let node = nodes.next().unwrap();
    match node.as_rule() {
        Rule::boolean => BooleanExpression::Value(node.as_str() == "true"),
        Rule::operation => parse_operation(node.into_inner()),
        smt => panic!("unknown node type '{:?}'.", smt),
    }
}

fn parse_operation(mut nodes: Pairs<Rule>) -> BooleanExpression {
    let node = nodes.next().unwrap();
    let lh = match node.as_rule() {
        Rule::register8 => parse_source_register(&node),
        Rule::memory_address => parse_source_memory(&node),
        v => panic!("unexpected node '{:?}' here.", v),
    };
    let middle_node = nodes.next().unwrap();
    let node = nodes.next().unwrap();
    let rh = parse_value8(&node);
    match middle_node.as_str() {
        "=" => BooleanExpression::Equal(lh, rh),
        ">=" => BooleanExpression::GreaterOrEqual(lh, rh),
        ">" => BooleanExpression::StrictlyGreater(lh, rh),
        "<=" => BooleanExpression::LesserOrEqual(lh, rh),
        "<" => BooleanExpression::StrictlyLesser(lh, rh),
        "!=" => BooleanExpression::Different(lh, rh),
        v => panic!("unknown 8 bits provider {:?}", v),
    }
}

fn parse_source_register(node: &Pair<Rule>) -> Source8 {
    match node.as_str() {
        "A" => Source8::Accumulator,
        "X" => Source8::RegisterX,
        "Y" => Source8::RegisterY,
        "S" => Source8::RegisterS,
        "SP" => Source8::RegisterSP,
        v => panic!("unknown register type '{:?}'.", v),
    }
}

fn parse_memory(addr: String) -> usize {
    let string = addr.clone();
    let bytes = match hex::decode(string) {
        Ok(s) => s,
        Err(t) => panic!("Could not turn '{}' into hex. {:?}", addr, t),
    };
    let mut addr: usize = 0;

    for byte in bytes.iter() {
        addr = addr << 8 | (*byte as usize);
    }

    addr
}

fn parse_source_memory(node: &Pair<Rule>) -> Source8 {
    let addr = parse_memory(node.as_str()[3..].to_owned());
    Source8::Memory(addr)
}

fn parse_value8(node: &Pair<Rule>) -> u8 {
    let hexa = node.as_str()[2..].to_owned();
    let val = hex::decode(hexa).unwrap();

    val[0] as u8
}

struct CommandLineCompleter {}

impl rustyline::completion::Completer for CommandLineCompleter {
    type Candidate = String;
    fn complete(&self, line: &str, pos: usize, _ctx: &Context) -> RustyResult<(usize, Vec<Self::Candidate>)> {
        let mut candidates:Vec<String> = vec![];
        let keywords = vec!["registers", "memory", "run", "disassemble", "help"];

        for word in keywords {
            if word.contains(line) {
                candidates.push(word.to_owned());
            }
        }

        if candidates.len() > 0 {
            Ok((0, candidates))
        } else {
            Ok((pos, vec![]))
        }
    }
}

impl rustyline::hint::Hinter for CommandLineCompleter { }

impl rustyline::highlight::Highlighter for CommandLineCompleter { }

impl rustyline::Helper for CommandLineCompleter { }
