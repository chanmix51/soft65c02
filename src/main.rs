/*
 * This is the CLI frontend for the Soft65C02 library.
 */
use ansi_term::Colour;

extern crate pest;
#[macro_use]
extern crate pest_derive;

use pest::error::Error as PestError;
use pest::iterators::{Pair, Pairs};
use pest::{Parser, RuleType};

extern crate rustyline;

use rustyline::error::ReadlineError;
use rustyline::Result as RustyResult;
use rustyline::{Context, Editor};

use soft65c02::{AddressableIO, LogLine, Memory, MemoryParserIterator, Registers, INIT_VECTOR_ADDR};
use soft65c02::memory::{little_endian, MiniFBMemory, MemoryError };

use structopt::StructOpt;

use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::fmt;


const VERSION: &'static str = "1.0.0-alpha2";

#[derive(StructOpt, Debug)]
#[structopt(name = "soft65C02")]
struct CommandLineArguments {
    // logline buffer is the number of log lines kept after each instruction execution
    #[structopt(short="l", long, default_value = "35")]
    logline_buffer: usize,

    // do not use history
    #[structopt(short="s", long)]
    no_history: bool,

    // do not create initial 64K RAM
    #[structopt(short="r", long)]
    no_ram: bool,
}

#[derive(Debug)]
pub struct ConfigToken {
    cli_opts: CommandLineArguments,
    ctrlc: Arc<AtomicBool>,
}

impl ConfigToken {
    fn new(cli_opts: CommandLineArguments, ctrlc: Arc<AtomicBool>) -> ConfigToken {
        ConfigToken {
            cli_opts,
            ctrlc,
        }
    }
}

#[derive(Parser)]
#[grammar = "cli.pest"]
pub struct BEParser;

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
            Source::RegisterCP => write!(f, "S"),
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
        }
    }
}

fn display_error<T: RuleType>(err: PestError<T>) {
    let (mark_str, msg) = match err.location {
        pest::error::InputLocation::Pos(x) => {
            let mut pos_str = String::new();
            for _ in 0..x {
                pos_str.push(' ');
            }
            pos_str.push('↑');

            (pos_str, format!("at position {}", x))
        }
        pest::error::InputLocation::Span((a, b)) => {
            let mut pos_str = String::new();
            for _ in 0..a {
                pos_str.push(' ');
            }
            pos_str.push('↑');
            for _ in a..b {
                pos_str.push(' ');
            }
            pos_str.push('↑');
            (
                pos_str,
                format!("somewhere between position {} and {}", a, b),
            )
        }
    };
    eprintln!("   {}", mark_str);
    print_err(&msg);
    match err.variant {
        pest::error::ErrorVariant::ParsingError {
            positives,
            negatives: _,
        } => {
            eprintln!(
                "{}",
                Colour::Fixed(240).paint(format!("hint: expected {:?}", positives))
            );
        }
        pest::error::ErrorVariant::CustomError { message } => {
            eprintln!(
                "{}",
                Colour::Fixed(240).paint(format!("message: {}", message))
            );
        }
    };
}

fn main() {
    // 0 global configuration
    let mut token = {
        let cli_opts = CommandLineArguments::from_args();
        let interrupted = Arc::new(AtomicBool::new(false));
        let rmtint = interrupted.clone();
        ctrlc::set_handler(move || {
            rmtint.store(true, Ordering::SeqCst);
        })
        .unwrap();
        ConfigToken::new(cli_opts, interrupted)
    };

    // 1 setting up memory & registers
    let mut registers = Registers::new(0x0000);
    let mut memory = if token.cli_opts.no_ram {
        Memory::new()
    } else {
        Memory::new_with_ram()
    };

    // 2 CLI prompt & readline configuration
    println!(
        "{}",
        Colour::Green.paint(format!("Welcome in Soft-65C02 version {}", VERSION))
    );
    let prompt = format!("{}", Colour::Fixed(148).bold().paint(">> "));
    let mut rl = Editor::<CommandLineCompleter>::new();
    if !token.cli_opts.no_history {
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }
    } else {
        println!("Command history disabled.");
    }
    rl.set_helper(Some(CommandLineCompleter {}));


    // 4 main CLI loop
    loop {
        let readline = rl.readline(&prompt);
        match readline {
            Ok(line) => {
                if line.len() == 0 {
                    continue;
                }
                if !token.cli_opts.no_history {
                    rl.add_history_entry(line.as_str());
                }
                match BEParser::parse(Rule::sentence, line.as_str()) {
                    Ok(mut pairs) => {
                        parse_instruction(
                            pairs.next().unwrap().into_inner(),
                            &mut registers,
                            &mut memory,
                            &token,
                        );
                        if token.ctrlc.load(Ordering::Relaxed) {
                            println!("Execution interrupted by CTRL+C!");
                            token.ctrlc.store(false, Ordering::SeqCst);
                        }
                    }
                    Err(parse_err) => {
                        display_error(parse_err);
                    }
                };
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL+C caught, press CTRL+D to exit.");
            }
            Err(ReadlineError::Eof) => {
                println!("Quit!");
                break;
            }
            Err(err) => {
                print_err(format!("{:?}", err).as_str());
                break;
            }
        }
    }
    if !token.cli_opts.no_history {
        rl.save_history("history.txt").unwrap();
        println!("Writing commands history in 'history.txt'.");
    }
}

pub fn parse_instruction(
    mut nodes: Pairs<Rule>,
    registers: &mut Registers,
    memory: &mut Memory,
    token: &ConfigToken,
) {
    if let Some(node) = nodes.next() {
        match node.as_rule() {
            Rule::registers_instruction =>
                exec_register_instruction(node.into_inner(), registers),
            Rule::memory_instruction =>
                exec_memory_instruction(node.into_inner(), memory, token),
            Rule::run_instruction =>
                exec_run_instruction(node.into_inner(), registers, memory, token),
            Rule::help_instruction => help(node.into_inner()),
            Rule::disassemble_instruction =>
                exec_disassemble_instruction(node.into_inner(), registers, memory, token),
            Rule::assert_instruction =>
                exec_assert_instruction(node.into_inner(), registers, memory),
            smt => { println!("{:?}", smt); },
        };
    }
}

fn exec_run_instruction(
    mut nodes: Pairs<Rule>,
    registers: &mut Registers,
    memory: &mut Memory,
    token: &ConfigToken,
) {
    let mut stop_condition = BooleanExpression::Value(true);
    while let Some(node) = nodes.next() {
        match node.as_rule() {
            Rule::memory_address => 
                registers.command_pointer = parse_memory(node.as_str()[3..].to_owned()),
            Rule::boolean_condition => stop_condition = parse_boolex(node.into_inner()),
            Rule::init_vector =>
                registers.command_pointer = little_endian(memory.read(INIT_VECTOR_ADDR, 2).unwrap()),
            _ => {}
        };
    }

    let mut cp = registers.command_pointer;
    let mut loglines: VecDeque<LogLine> = VecDeque::new();
    let mut i = 0;
    loop {
        loglines.push_back(soft65c02::execute_step(registers, memory).unwrap());
        i += 1;
        if loglines.len() > token.cli_opts.logline_buffer {
            loglines.pop_front();
        }
        if token.ctrlc.load(Ordering::Relaxed)
            || stop_condition.solve(registers, memory)
            || registers.command_pointer == cp
        {
            break;
        }
        cp = registers.command_pointer;
    }

    if i > token.cli_opts.logline_buffer {
        println!("Stopped after {} cpu instructions.", i);
    }
    loglines.iter().for_each(|x| println!("{}", x));
}

fn exec_disassemble_instruction(
    mut nodes: Pairs<Rule>,
    registers: &Registers,
    memory: &Memory,
    token: &ConfigToken,
) {
    let mut addr = registers.command_pointer;
    let mut len = 0;
    while let Some(node) = nodes.next() {
        match node.as_rule() {
            Rule::memory_address => addr = parse_memory(node.as_str()[3..].to_owned()),
            Rule::size_parameter => len = node.as_str().parse::<usize>().unwrap(),
            _ => {}
        }
    }

    if len == 0 {
        print_err("length 0");
        return;
    }

    for (op, line) in MemoryParserIterator::new(addr, &memory).enumerate() {
        println!("{}", line);
        if token.ctrlc.load(Ordering::Relaxed) || op >= len {
            break;
        }
    }
}


fn exec_assert_instruction(mut nodes: Pairs<Rule>, registers: &Registers, memory: &Memory) {
    let condition = parse_boolex(nodes.next().unwrap().into_inner());
    if !condition.solve(registers, memory) {
        panic!("{} is not true", condition);
    } else {
        println!("ok");
    }
}

fn exec_memory_instruction(
    mut nodes: Pairs<Rule>,
    memory: &mut Memory,
    token: &ConfigToken,
) {
    let node = nodes.next().unwrap();
    match node.as_rule() {
        Rule::memory_show => {
            let mut subnodes = node.into_inner();
            let addr = parse_memory(subnodes.next().unwrap().as_str()[3..].to_owned());
            let len: usize = subnodes.next().unwrap().as_str().parse::<usize>().unwrap();
            match mem_dump(addr, len, memory) {
                Ok(lines) => {
                    for line in lines.iter() {
                        println!("{}", line);
                        if token.ctrlc.load(Ordering::Relaxed) {
                            break;
                        }
                    }
                },
                Err(e)  => print_err(format!("memory error: {}", e).as_str()),
            }
        }
        Rule::memory_load => {
            let mut subnodes = node.into_inner();
            let addr = parse_memory(subnodes.next().unwrap().as_str()[3..].to_owned());
            let filename = subnodes.next().unwrap().as_str().trim_matches('"');
            match load_memory(filename, addr, memory) {
                Ok(len) => println!("Loaded {} bytes at address #0x{:04X}.", len, addr),
                Err(e) => print_err(format!("{}", e).as_str()),
            }
        },
        Rule::memory_sub_list => {
            let subs = memory.get_subsystems_info();
            if subs.len() == 0 {
                println!("No subsystem loaded.");
            } else {
                println!("Memory subsystems:");
                for line in subs {
                    println!("{}", line);
                }
            }
        }
        Rule::memory_sub_add => {
            let mut nodes = node.into_inner();
            let subnode = nodes.next().unwrap();
            let addr = parse_memory(subnode.as_str()[3..].to_owned());
            let subnode = nodes.next().unwrap();
            match subnode.as_str() {
                "minifb"    => memory.add_subsystem("FRAMEBUFFER", addr, MiniFBMemory::new(None)),
                whatever    => print_err(format!("unknown sub system {}", whatever).as_str()),
            }
        }
        Rule::memory_write => {
            let mut nodes = node.into_inner();
            let addr_node = nodes.next().unwrap();
            let addr = parse_memory(addr_node.as_str()[3..].to_owned());
            let bytes_node = nodes.next().unwrap();
            let bytes = parse_bytes(bytes_node.as_str());
            match memory.write(addr, &bytes) {
                Ok(_)   => println!("{} bytes written", bytes.len()),
                Err(e)  => print_err(format!("{}", e).as_str()),
            }
        }
        _ => println!("{:?}", node),
    }
}

fn mem_dump(start: usize, len: usize, memory: &Memory) -> Result<Vec<String>, MemoryError> {
    let mut output:Vec<String> = vec![];
    if len == 0 { return Ok(output) }
    let address = start - (start % 16);
    let bytes = memory.read(address, 16 * len)?;

    for lineno in 0..len {
        let mut line = format!("#{:04X}: ", address + lineno * 16);
        for col in 0..16 {
            if col == 8 {
                line.push(' ');
            }
            line = format!("{} {:02x}", line, bytes[16 * lineno + col]);
        }
        output.push(line);
    }

    Ok(output)
}

fn load_memory(filename: &str, addr: usize, memory: &mut Memory) -> std::io::Result<usize> {
    let buffer = {
        let mut f = File::open(filename)?;
        let mut buffer: Vec<u8> = vec![];
        f.read_to_end(&mut buffer)?;
        buffer
    };
    memory.write(addr, &buffer).unwrap();

    Ok(buffer.len())
}

fn exec_register_instruction(mut nodes: Pairs<Rule>, registers: &mut Registers) {
    let node = nodes.next().unwrap();
    match node.as_rule() {
        Rule::registers_show => {
            println!("{:?}", registers);
        }
        Rule::registers_flush => {
            registers.flush();
            println!("Registers flushed!");
        }
        _ => {
            println!("{:?}", node);
        }
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
            }
            Rule::help_memory => {
                println!("{}", Colour::Green.paint("Memory commands:"));
                println!("  memory show ADDRESS LENGTH");
                println!("          Show the content of the memory starting from ADDRESS.");
                print_example("memory show #0x1234 100");
                println!("");
                println!("   memory load ADDRESS \"filename.ext\" ");
                println!("          Load a binary file at the selected address in memory.");
                println!("          The content of the file is copied in the memory, so the memory has to");
                println!("          be writable.");
                print_example("memory load #0x1C00 \"program.bin\"");
                println!("");
                println!("   memory write ADDRESS 0x(BYTES)");
                println!("          Write bytes at the given address.");
                print_example("memory write #0x0200 0x(0a,11,00,fe)");
                println!("");
                println!("  memory sub list");
                println!("          Show the list of the running memory subsystems.");
                println!("");
                println!("  memory sub add ADDRESS SUBSYSTEM");
                println!("          Add the given memory subsystem starting at ADDRESS.");
                println!("          For now, only “minifb” is implemented.");
                print_example("memory add sub #0x0200 minifb");
            }
            Rule::help_run => {
                println!("{}", Colour::Green.paint("Execution commands:"));
                println!("   run [ADDRESS] [until BOOLEAN_CONDITION]");
                println!("          Launch execution of the program. If an address is given, the");
                println!("          instruction at this address is executed otherwise the instruction");
                println!("          pointed by the current CP register is executed. Since this");
                println!("          register is automatically updated by each instruction, it is");
                println!("          possible either to run programs step by step or continuously");
                println!("          until a certain condition is met. Without condition information,");
                println!("          this executes only one instruction.");
                println!("");
                println!("{}", Colour::White.bold().paint("Examples:"));
                print_example("run");
                println!("          Execute the next instruction at the actual CP register position.");
                println!("");
                print_example("run #0x1C00");
                println!("          Execute the instruction at #0x1C00.");
                println!("");
                print_example("run init");
                println!("          Load CP with the init vector (#0xFFFC) and run the first");
                println!("          instruction.");
                println!("");
                println!("{}", Colour::Green.paint("Boolean conditions"));
                println!(
                    "          By default, only one instruction is executed, but it is possible"
                );
                println!("          to provide a custom condition so a program can be executed until");
                println!("          a certain state is met. These conditions can be made on either");
                println!("          registers or memory content");
                println!(
                    "          In any cases, the program will stop if the Command Pointer is not"
                );
                println!("          incremented after an instruction. This is the case for the STP");
                println!("          (stop) instruction but also after  infinite loops like BRA -2 or");
                println!("          a JMP at the exact same address.");
                println!("");
                println!("{}", Colour::White.bold().paint("Examples:"));
                print_example("run init until false");
                println!("          Init CP and launch the program forever. This may require CTRL-C to");
                println!("          break.");
                println!("");
                print_example("run #0x0400 until A <= 0x12");
                println!("          The execution is launched starting at #0x0400 until the A register");
                println!("          is lesser or equal to 0x12.");
                println!("");
                print_example("run until #0x0200 > 0x00");
                println!("          The execution is continued until the given memory address holds a");
                println!("          value greater than 0.");
                println!("");
                print_example("run until S > 0x7f");
                println!(
                    "          The execution is continued until the Negative flag of the status"
                );
                println!("          register is set.");
                println!("");
                print_example("run until CP = 0x1234");
                println!(
                    "          The execution is continued until the Command Pointer equals the"
                );
                println!("          given value.");
            }
            Rule::help_disassemble => {
                println!("{}", Colour::Green.paint("Registers command:"));
                println!("");
                println!("  disassemble [ADDRESS] LENGTH");
                println!("          Disassemble starting from ADDRESS for the next \"OPERATIONS\"");
                println!(
                    "          instructions. If the ADDRESS parameter is not provided, the actual"
                );
                println!("          register Command Pointer's value is taken.");
                println!("");
                print_example("disassemble #0x1C00 100");
                println!("          Disassemble 100 opcodes starting from address 0x1C00.");
                println!("");
                print_example("disassemble 10");
                println!(
                    "          Disassemble 10 opcodes starting from the address in register CP."
                );
            }
            Rule::help_assert => {
                println!("{}", Colour::Green.paint("Assertion command:"));
                println!("");
                println!("  assert BOOLEAN_CONDITION");
                println!("          Evaluate the boolean condition. A \"ok\" message is printed if");
                println!("          the condition is true, the program exit with an error code");
                println!("          otherwise. See the \"run until\" command to get more explanations");
                println!("          about the boolean conditions.");
                println!("");
                print_example("assert #0x0200 = 0x1a");
                println!("          Test this memory address has got the given value.");
                println!("");
                print_example("assert X >= 0x80");
                println!("          Test the X register is bigger than 0x80.");
            }
            _ => {}
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
        println!("   memory write ADDRESS 0x(BYTES)");
        println!("          Write bytes starting at the given address in memory.");
        println!("{}", Colour::White.bold().paint("Execution"));
        println!("   run [ADDRESS] [until BOOLEAN_CONDITION]");
        println!("          Launch execution of the program.");
        println!("          If the ADDRESS parameter is not provided, the actual register Command");
        println!("          Pointer value is taken. If no conditions are given, this executes one");
        println!("          instruction and get back to interactive mode (step by step mode).");
        println!("{}", Colour::White.bold().paint("Disassembler"));
        println!("   disassemble [ADDRESS] OPERATIONS");
        println!(
            "         Disassemble starting from ADDRESS for the next \"OPERATIONS\" instructions."
        );
        println!("{}", Colour::White.bold().paint("Asserter"));
        println!("   assert BOOLEAN_CONDITION");
        println!("          If the assertion is true, a 'ok' message is printed otherwise the program");
        println!("          panics and exit with an error code. This is intended for automated tests.");
        println!("{}", Colour::White.bold().paint("Help"));
        println!("   help [TOPIC]");
        println!("          Display informations about commands.");
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
        Rule::register8 | Rule::register16 => parse_source_register(&node),
        Rule::memory_address => parse_source_memory(&node),
        v => panic!("unexpected node '{:?}' here.", v),
    };
    let middle_node = nodes.next().unwrap();
    let node = nodes.next().unwrap();
    let rh = parse_value(&node);
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

fn parse_source_register(node: &Pair<Rule>) -> Source {
    match node.as_str() {
        "A" => Source::Accumulator,
        "X" => Source::RegisterX,
        "Y" => Source::RegisterY,
        "S" => Source::RegisterS,
        "SP" => Source::RegisterSP,
        "CP" => Source::RegisterCP,
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

fn parse_source_memory(node: &Pair<Rule>) -> Source {
    let addr = parse_memory(node.as_str()[3..].to_owned());
    Source::Memory(addr)
}

fn parse_value(node: &Pair<Rule>) -> usize {
    let hexa = node.as_str()[2..].to_owned();
    parse_memory(hexa)
}

fn parse_bytes(bytes: &str) -> Vec<u8> {
    bytes.split(',')
        .map(|x| hex::decode(x.trim()).unwrap()[0])
        .collect()
}

fn print_err(msg: &str) {
    eprintln!("{}: {}", Colour::Red.paint("Error"), msg);
}

fn print_example(msg: &str) {
    println!(
        "          Example: {}",
        Colour::Fixed(130).paint(msg)
    );
}

struct CommandLineCompleter {}

impl rustyline::completion::Completer for CommandLineCompleter {
    type Candidate = String;
    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context,
    ) -> RustyResult<(usize, Vec<Self::Candidate>)> {
        let mut candidates: Vec<String> = vec![];
        let keywords = vec![
            "registers show",
            "registers flush",
            "memory",
            "memory show #0x",
            "memory load #0x",
            "memory write #0x",
            "memory sub list",
            "memory sub add #0x",
            "run ",
            "run #0x",
            "run until ",
            "assert ",
            "disassemble ",
            "disassemble #0x",
            "help",
            "help registers",
            "help memory",
            "help run",
            "help disassemble",
        ];

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

impl rustyline::hint::Hinter for CommandLineCompleter {}

impl rustyline::highlight::Highlighter for CommandLineCompleter {}

impl rustyline::Helper for CommandLineCompleter {}
