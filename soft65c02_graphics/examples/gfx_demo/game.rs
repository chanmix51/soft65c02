/*
 * Multi-Game Graphics Demo for Soft65C02
 * 
 * This demo showcases a sophisticated multi-game system that combines:
 * - 6502 assembly for real-time input handling and mode switching
 * - Rust acceleration for computationally intensive graphics generation
 * - Memory-mapped communication for efficient DMA-style operation
 * 
 * Architecture Features:
 * - State preservation across mode switches (games maintain internal state)
 * - Extensible GameProcessor trait for easy addition of new games
 * - International keyboard layout compatibility 
 * - 600 Hz update rate with efficient memory management
 * 
 * Current Games:
 * - Mode 1: Conway's Game of Life with colorful neighbor-count visualization
 * - Mode 2: Interactive Mandelbrot set explorer with real-time parameter adjustment
 * - Modes 3-9: Reserved for future games and visualizations
 * 
 */

use std::io::prelude::*;
use std::fs::File;
use std::time::{Duration, Instant};
use std::sync::mpsc::channel;

use soft65c02_graphics::PixelsDisplay;
use soft65c02_lib::{AddressableIO, Memory, Registers, execute_step};
use soft65c02_tester::{CliDisplayer, Displayer};

mod gol;
mod mandlebrot;
mod sfc;
mod help;
use gol::{GameOfLifeState, get_gol_palette};
use mandlebrot::{MandelbrotState, get_mandelbrot_palette};
use sfc::{SpaceFillingCurveState, get_sfc_palette};
use help::{HelpScreenState, get_help_palette};

// Frame rate constants
const UPDATES_PER_SECOND: u64 = 600;
const FRAME_TIME: Duration = Duration::from_micros(1_000_000 / UPDATES_PER_SECOND);

// Memory-mapped locations (relative to display start at 0x8000)
const DISPLAY_START: usize = 0x8000;
const KEYBOARD_INPUT: usize = DISPLAY_START + 0x30;    // 0x8030 - keyboard input buffer
const COMMAND_ADDR: usize = DISPLAY_START + 0x40;      // 0x8040 - command location
const MODE_ADDR: usize = DISPLAY_START + 0x41;         // 0x8041 - mode location

const APPLICATION_LOAD_START: usize = 0x1000;

// Game modes
const MODE_NO_OP: u8 = 0x00;
const MODE_HELP: u8 = 0x01;
const MODE_GAME_OF_LIFE: u8 = 0x02;
const MODE_MANDELBROT: u8 = 0x03;
const MODE_SPACE_FILLING_CURVE: u8 = 0x04;

// Commands
const CMD_NO_ACTION: u8 = 0x00;
const CMD_GENERATE: u8 = 0x01;
const CMD_PROCESS_KEYBOARD: u8 = 0x02;
const CMD_DEBUG_HALT: u8 = 0x03;

// Game processor trait that all games must implement
trait GameProcessor {
    fn compute_next_generation(&mut self);
    fn write_to_memory(&mut self, memory: &mut Memory);
    fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool;
    fn get_palette(&self) -> Vec<u8>;
}

// Implement the trait for GameOfLifeState
impl GameProcessor for GameOfLifeState {
    fn compute_next_generation(&mut self) {
        self.compute_next_generation();
    }
    
    fn write_to_memory(&mut self, memory: &mut Memory) {
        self.write_to_memory(memory);
    }
    
    fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool {
        self.process_keyboard_input(key_code, memory)
    }
    
    fn get_palette(&self) -> Vec<u8> {
        get_gol_palette()
    }
}

// Implement the trait for MandelbrotState
impl GameProcessor for MandelbrotState {
    fn compute_next_generation(&mut self) {
        self.compute_next_generation();
    }
    
    fn write_to_memory(&mut self, memory: &mut Memory) {
        self.write_to_memory(memory);
    }
    
    fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool {
        self.process_keyboard_input(key_code, memory)
    }
    
    fn get_palette(&self) -> Vec<u8> {
        get_mandelbrot_palette()
    }
}

// Implement the trait for SpaceFillingCurveState
impl GameProcessor for SpaceFillingCurveState {
    fn compute_next_generation(&mut self) {
        self.compute_next_generation();
    }
    
    fn write_to_memory(&mut self, memory: &mut Memory) {
        self.write_to_memory(memory);
    }
    
    fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool {
        self.process_keyboard_input(key_code, memory)
    }
    
    fn get_palette(&self) -> Vec<u8> {
        get_sfc_palette()
    }
}

// Implement the trait for HelpScreenState
impl GameProcessor for HelpScreenState {
    fn compute_next_generation(&mut self) {
        self.compute_next_generation();
    }
    
    fn write_to_memory(&mut self, memory: &mut Memory) {
        self.write_to_memory(memory);
    }
    
    fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool {
        self.process_keyboard_input(key_code, memory)
    }
    
    fn get_palette(&self) -> Vec<u8> {
        get_help_palette()
    }
}

// Game manager that holds all game states and dispatches commands
struct GameManager {
    help_screen: Option<HelpScreenState>,
    game_of_life: Option<GameOfLifeState>,
    mandelbrot: Option<MandelbrotState>,
    space_filling_curve: Option<SpaceFillingCurveState>,
    current_mode: u8,
}

impl GameManager {
    fn new() -> Self {
        Self {
            help_screen: None,
            game_of_life: None,
            mandelbrot: None,
            space_filling_curve: None,
            current_mode: MODE_NO_OP,
        }
    }
    
    fn get_or_create_game_processor(&mut self, mode: u8, memory: &mut Memory) -> Option<&mut dyn GameProcessor> {
        match mode {
            MODE_HELP => {
                if self.help_screen.is_none() {
                    println!("Creating Help Screen");
                    let mut help_state = HelpScreenState::new();
                    help_state.render_help_screen();
                    help_state.write_to_memory(memory);
                    self.help_screen = Some(help_state);
                }
                self.help_screen.as_mut().map(|h| h as &mut dyn GameProcessor)
            }
            MODE_GAME_OF_LIFE => {
                if self.game_of_life.is_none() {
                    println!("Creating Game of Life processor with initial random pattern");
                    let mut gol_state = GameOfLifeState::new();
                    gol_state.reset_to_random(memory);  // Only initialize when first created
                    self.game_of_life = Some(gol_state);
                }
                self.game_of_life.as_mut().map(|g| g as &mut dyn GameProcessor)
            }
            MODE_MANDELBROT => {
                if self.mandelbrot.is_none() {
                    println!("Creating Mandelbrot processor with default view");
                    let mut mandelbrot_state = MandelbrotState::new();
                    mandelbrot_state.reset_to_default(memory);  // Only initialize when first created
                    self.mandelbrot = Some(mandelbrot_state);
                }
                self.mandelbrot.as_mut().map(|m| m as &mut dyn GameProcessor)
            }
            MODE_SPACE_FILLING_CURVE => {
                if self.space_filling_curve.is_none() {
                    println!("Creating space-filling curve processor with default parameters");
                    let mut sfc_state = SpaceFillingCurveState::new();
                    sfc_state.reset_to_default(memory);  // Only initialize when first created
                    self.space_filling_curve = Some(sfc_state);
                }
                self.space_filling_curve.as_mut().map(|h| h as &mut dyn GameProcessor)
            }
            _ => None,
        }
    }
    
    fn switch_mode(&mut self, new_mode: u8, memory: &mut Memory) {
        if new_mode != self.current_mode {
            // println!("Mode changed from 0x{:02X} to 0x{:02X}", self.current_mode, new_mode);
            self.current_mode = new_mode;
            
            // Set palette for the new mode
            if let Some(processor) = self.get_or_create_game_processor(new_mode, memory) {
                let palette = processor.get_palette();
                memory.write(DISPLAY_START, &palette).unwrap();
            }
        }
    }
    
    fn process_command(&mut self, command: u8, memory: &mut Memory) {
        match command {
            CMD_GENERATE => {
                if let Some(processor) = self.get_or_create_game_processor(self.current_mode, memory) {
                    // Don't load from memory - games maintain their own internal state
                    processor.compute_next_generation();
                    processor.write_to_memory(memory);
                }
            }
            CMD_PROCESS_KEYBOARD => {
                if let Ok(key_data) = memory.read(KEYBOARD_INPUT, 1) {
                    if !key_data.is_empty() && key_data[0] != 0 {
                        let key_code = key_data[0];
                        println!("Processing key: 0x{:02X}", key_code);
                        
                        if let Some(processor) = self.get_or_create_game_processor(self.current_mode, memory) {
                            let should_generate = processor.process_keyboard_input(key_code, memory);
                            if should_generate {
                                // Don't load from memory - games maintain their own internal state
                                processor.compute_next_generation();
                                processor.write_to_memory(memory);
                            }
                        }
                    }
                                 }
             }
             CMD_DEBUG_HALT => {
                 println!("Debug halt - press Enter to continue...");
                 let mut input = String::new();
                 std::io::stdin().read_line(&mut input).unwrap();
             }
             _ => {} // Unknown command, ignore
         }
    }
}

fn main() {
    let init_vector: usize = APPLICATION_LOAD_START;  // Start program in low memory
    let mut memory = Memory::new_with_ram();
    let display = PixelsDisplay::new();
    memory.add_subsystem("VIDEO DISPLAY", DISPLAY_START, display);
    
    // Initialize game manager
    let mut game_manager = GameManager::new();
    
    // Try to load compiled binary
    let program = match load_program_binary("build/game.bin") {
        Ok(data) => {
            println!("Loaded game binary ({} bytes)", data.len());
            data
        }
        Err(_) => {
            // Try the old filename for backward compatibility
            match load_program_binary("build/gol.bin") {
                Ok(data) => {
                    println!("Loaded Game of Life binary ({} bytes)", data.len());
                    data
                }
                Err(_) => {
                    println!("Binary not found, using built-in demo program");
                    create_demo_program()
                }
            }
        }
    };
    
    // Load program after initializing display
    memory.write(init_vector, &program).unwrap();
    
    // Default start address
    let start_addr = APPLICATION_LOAD_START;

    let (sender, receiver) = channel();
    let mut displayer = CliDisplayer::new(std::io::stdout(), true);
    let display_thread = std::thread::spawn(move || {
        displayer.display(receiver).unwrap();
    });

    let mut registers = Registers::new(start_addr);
    let mut cycle = 0;

    println!("Starting memory-mapped game processor...");
    println!("Running at {} updates per second", UPDATES_PER_SECOND);
    println!("Close the window to exit.");
    println!("Using memory-mapped architecture:");
    println!("  Command Address: 0x{:04X}", COMMAND_ADDR);
    println!("  Mode Address:    0x{:04X}", MODE_ADDR);
    println!("  Mode values: 0x{:02X}=Help, 0x{:02X}=Game of Life, 0x{:02X}=Mandelbrot, 0x{:02X}=Space-filling curves",
             MODE_HELP, MODE_GAME_OF_LIFE, MODE_MANDELBROT, MODE_SPACE_FILLING_CURVE);
    println!("  Command values: 0x{:02X}=No action, 0x{:02X}=Generate step, 0x{:02X}=Process keyboard, 0x{:02X}=Debug halt", 
             CMD_NO_ACTION, CMD_GENERATE, CMD_PROCESS_KEYBOARD, CMD_DEBUG_HALT);

    game_manager.process_command(CMD_GENERATE, &mut memory);

    loop {
        let frame_start = Instant::now();

        // Check memory-mapped commands
        if let Ok(command_data) = memory.read(COMMAND_ADDR, 1) {
            if !command_data.is_empty() {
                let command = command_data[0];
                
                if command != CMD_NO_ACTION {
                    // Check for mode changes first
                    if let Ok(mode_data) = memory.read(MODE_ADDR, 1) {
                        if !mode_data.is_empty() {
                            let mode = mode_data[0];
                            game_manager.switch_mode(mode, &mut memory);
                        }
                    }
                    
                    // Process the command
                    game_manager.process_command(command, &mut memory);
                    
                    // Clear the command after processing
                    memory.write(COMMAND_ADDR, &[CMD_NO_ACTION]).unwrap();
                }
            }
        }

        // Execute one CPU instruction
        if let Ok(_instruction) = execute_step(&mut registers, &mut memory) {
            cycle += 1;
        } else {
            println!("Game simulation ended after {} cycles", cycle);
            break;
        }
        
        // Frame rate limiting
        let frame_time = frame_start.elapsed();
        if frame_time < FRAME_TIME {
            std::thread::sleep(FRAME_TIME - frame_time);
        }
    }
    
    // Drop the sender to signal the display thread to exit
    drop(sender);
    display_thread.join().unwrap();
    
    println!("Game simulation ended after {} cycles", cycle);
}

fn load_program_binary(filename: &str) -> Result<Vec<u8>, std::io::Error> {
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    Ok(buffer)
}

fn create_demo_program() -> Vec<u8> {
    // hard coded version of game.s
    // generated with:
    //   hexdump -v -e '8/1 "0x%02x, " "\n"' build/game.bin
    vec![
        0xa9, 0x2c, 0x8d, 0xfe, 0xff, 0x8d, 0xfa, 0xff,
        0xa9, 0x10, 0x8d, 0xff, 0xff, 0x8d, 0xfb, 0xff,
        0xa9, 0x00, 0x8d, 0xfc, 0xff, 0xa9, 0x10, 0x8d,
        0xfd, 0xff, 0xa9, 0xff, 0xa2, 0x7f, 0x85, 0x82,
        0x86, 0x83, 0xa2, 0xff, 0x9a, 0xe8, 0x8a, 0xa8,
        0x18, 0x4c, 0x2d, 0x10, 0xdb, 0xa2, 0x00, 0x8e,
        0x40, 0x80, 0xe8, 0x8e, 0x41, 0x80, 0x20, 0x46,
        0x10, 0xad, 0x7a, 0x10, 0xd0, 0xf8, 0xa9, 0x01,
        0x8d, 0x40, 0x80, 0x4c, 0x36, 0x10, 0xad, 0x30,
        0x80, 0xd0, 0x01, 0x60, 0xc9, 0x50, 0xf0, 0x18,
        0xc9, 0x70, 0xf0, 0x14, 0xc9, 0x30, 0x90, 0x1b,
        0xc9, 0x3a, 0xb0, 0x17, 0x38, 0xe9, 0x30, 0x8d,
        0x41, 0x80, 0xa9, 0x00, 0x8d, 0x30, 0x80, 0x60,
        0xad, 0x7a, 0x10, 0x49, 0x01, 0x8d, 0x7a, 0x10,
        0x4c, 0x62, 0x10, 0xa9, 0x02, 0x8d, 0x40, 0x80,
        0xd0, 0xe8, 0x00
    ]
} 
