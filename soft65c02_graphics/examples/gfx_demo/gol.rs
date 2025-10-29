/*
 * Conway's Game of Life Implementation
 * 
 * Features:
 * - Colorful visualization based on neighbor counts  
 * - Optimized brute-force algorithm (per Rokicki paper)
 * - 30% random initialization for interesting patterns
 * - Real-time evolution at 600 Hz with Rust acceleration
 * - Interactive reset functionality (R key)
 * 
 * Color Mapping:
 * - Dead cells: Black (color 0)
 * - Live cells: Blue gradient based on neighbor count (colors 1-8)
 * 
 * This creates beautiful flowing patterns as cellular automata evolve,
 * with color intensity representing local population density.
 */

use rand::Rng;
use soft65c02_lib::{Memory, AddressableIO};

// ASCII key codes from ReceivedCharacter events
const KEY_R: u8 = b'R';  // R key (ASCII 'R' = 0x52)

// Constants for Game of Life
const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 96;
const BYTES_PER_ROW: usize = SCREEN_WIDTH / 2;  // 2 pixels per byte
const VIDEO_BUFFER_START: usize = 0x8100;

// Game of Life lookup table - static constant to avoid recreating every call
const LIFE_RULES: [[u8; 9]; 2] = [
    // Dead cell (index 0): becomes alive only with exactly 3 neighbors
    [0, 0, 0, 1, 0, 0, 0, 0, 0],  // Birth with 3 neighbors -> alive (will be colored later)
    // Live cell (index 1): stays alive with 2 or 3 neighbors  
    [0, 0, 1, 1, 0, 0, 0, 0, 0],  // Survival with 2 or 3 neighbors -> alive
];

// Game of Life state using optimized brute force approach (per Rokicki paper)
// Now stores neighbor counts for colorful visualization
pub struct GameOfLifeState {
    current_generation: Vec<Vec<u8>>,  // 0 = dead, 1-8 = alive with N neighbors
    next_generation: Vec<Vec<u8>>,
    write_buffer: Vec<u8>,  // Reusable buffer for memory writes
    color_counts: [u32; 9],  // Reusable color counting array
    generation_count: u32,
}

impl GameOfLifeState {
    pub fn new() -> Self {
        Self {
            current_generation: vec![vec![0u8; SCREEN_WIDTH]; SCREEN_HEIGHT],
            next_generation: vec![vec![0u8; SCREEN_WIDTH]; SCREEN_HEIGHT],
            write_buffer: vec![0u8; BYTES_PER_ROW * SCREEN_HEIGHT],
            color_counts: [0u32; 9],
            generation_count: 0,
        }
    }
    
    pub fn reset_to_random(&mut self, memory: &mut Memory) {
        println!("🎲 Generating new random pattern...");
        
        // Generate new random pattern
        let mut rng = rand::rng();
        let mut buffer = vec![0u8; BYTES_PER_ROW * SCREEN_HEIGHT];
        let mut live_cells = 0;
        
        // Fill buffer with random bits (30% chance of life)
        for byte in buffer.iter_mut() {
            let pixel1 = if rng.random_bool(0.3) { 1 } else { 0 };
            let pixel2 = if rng.random_bool(0.3) { 1 } else { 0 };
            *byte = (pixel1 << 4) | pixel2;
            if pixel1 == 1 { live_cells += 1; }
            if pixel2 == 1 { live_cells += 1; }
        }
        
        println!("Generated pattern: {} live cells out of {} total ({:.1}%)", 
                 live_cells, SCREEN_WIDTH * SCREEN_HEIGHT,
                 (live_cells as f64 / (SCREEN_WIDTH * SCREEN_HEIGHT) as f64) * 100.0);
        
        // Write to video buffer
        memory.write(VIDEO_BUFFER_START, &buffer).unwrap();
        
        // Verify the write worked
        let read_back = memory.read(VIDEO_BUFFER_START, buffer.len()).unwrap();
        assert_eq!(buffer, read_back, "Buffer write verification failed!");
        
        // Load the new pattern into our state
        self.load_from_memory(memory);
        
        // Reset generation counter
        self.generation_count = 0;
    }
    
    pub fn load_from_memory(&mut self, memory: &Memory) {
        // Read current state from video buffer directly into 2D array
        for y in 0..SCREEN_HEIGHT {
            let row_offset = y * BYTES_PER_ROW;
            if let Ok(row_data) = memory.read(VIDEO_BUFFER_START + row_offset, BYTES_PER_ROW) {
                for x in 0..SCREEN_WIDTH {
                    let byte_index = x / 2;
                    let is_upper_nibble = (x % 2) == 1;
                    
                    if byte_index < row_data.len() {
                        let pixel_value = if is_upper_nibble {
                            (row_data[byte_index] >> 4) & 0x0F
                        } else {
                            row_data[byte_index] & 0x0F
                        };
                        
                        // Convert color back to alive/dead (non-zero = alive as color 1 initially)
                        self.current_generation[y][x] = if pixel_value != 0 { 1 } else { 0 };
                    }
                }
            }
        }
    }
    
    pub fn compute_next_generation(&mut self) {
        // Based on Rokicki's paper: simple optimized brute force is often faster 
        // than complex algorithms for this type of pattern
        self.compute_brute_force_optimized();
        self.generation_count += 1;
    }
    
    fn compute_brute_force_optimized(&mut self) {
        // Reset color counts (reusing array instead of allocating)
        self.color_counts.fill(0);
        
        // PASS 1: Apply Game of Life rules to determine which cells live/die
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let neighbors = self.count_neighbors_brute_force(x, y);
                let is_alive = if self.current_generation[y][x] > 0 { 1 } else { 0 };
                
                // Apply Game of Life rules using static lookup table
                let will_live = LIFE_RULES[is_alive][neighbors as usize] > 0;
                
                // Store just alive/dead for now
                self.next_generation[y][x] = if will_live { 1 } else { 0 };
            }
        }
        
        // PASS 2: Color the living cells based on their neighbor count in the NEW generation
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                if self.next_generation[y][x] > 0 {  // If cell is alive
                    // Count neighbors in the NEW generation
                    let neighbors = self.count_neighbors_in_next_generation(x, y);
                    let color = neighbors.max(1);  // Use neighbor count as color (min 1 for visibility)
                    self.next_generation[y][x] = color;
                    
                    if (color as usize) < self.color_counts.len() {
                        self.color_counts[color as usize] += 1;
                    }
                } else {
                    // Dead cells remain black (0)
                    self.color_counts[0] += 1;
                }
            }
        }
        
        std::mem::swap(&mut self.current_generation, &mut self.next_generation);
    }
    
    #[inline]
    fn count_neighbors(&self, grid: &[Vec<u8>], x: usize, y: usize) -> u8 {
        let mut count = 0;
        
        // Pre-calculate coordinates with wrapping for better performance
        let x_prev = if x == 0 { SCREEN_WIDTH - 1 } else { x - 1 };
        let x_next = if x == SCREEN_WIDTH - 1 { 0 } else { x + 1 };
        let y_prev = if y == 0 { SCREEN_HEIGHT - 1 } else { y - 1 };
        let y_next = if y == SCREEN_HEIGHT - 1 { 0 } else { y + 1 };
        
        // Check all 8 neighbors - unrolled for maximum performance
        if grid[y_prev][x_prev] > 0 { count += 1; }  // Top-left
        if grid[y_prev][x] > 0      { count += 1; }  // Top
        if grid[y_prev][x_next] > 0 { count += 1; }  // Top-right
        if grid[y][x_prev] > 0      { count += 1; }  // Left
        if grid[y][x_next] > 0      { count += 1; }  // Right
        if grid[y_next][x_prev] > 0 { count += 1; }  // Bottom-left
        if grid[y_next][x] > 0      { count += 1; }  // Bottom
        if grid[y_next][x_next] > 0 { count += 1; }  // Bottom-right
        
        count
    }
    
    #[inline]
    fn count_neighbors_brute_force(&self, x: usize, y: usize) -> u8 {
        self.count_neighbors(&self.current_generation, x, y)
    }
    
    #[inline]
    fn count_neighbors_in_next_generation(&self, x: usize, y: usize) -> u8 {
        self.count_neighbors(&self.next_generation, x, y)
    }
    
    pub fn write_to_memory(&mut self, memory: &mut Memory) {
        // Clear reusable buffer instead of allocating new one
        self.write_buffer.fill(0);
        
        // Write all pixels from 2D array using color values directly
        for y in 0..SCREEN_HEIGHT {
            let row_offset = y * BYTES_PER_ROW;
            for x in 0..SCREEN_WIDTH {
                let byte_index = row_offset + (x / 2);
                let is_upper_nibble = (x % 2) == 1;
                let pixel_value = self.current_generation[y][x]; // Already contains color index
                
                if byte_index < self.write_buffer.len() {
                    if is_upper_nibble {
                        self.write_buffer[byte_index] |= pixel_value << 4;
                    } else {
                        self.write_buffer[byte_index] |= pixel_value;
                    }
                }
            }
        }
        
        // Write directly to video buffer
        memory.write(VIDEO_BUFFER_START, &self.write_buffer).unwrap();
    }
    
    pub fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool {
        // Handle keyboard input - accept both upper and lower case for commands
        match key_code {
            KEY_R | b'r' => {
                // R/r key pressed - reset the game with new random pattern
                println!("🎲 Reset key pressed - generating new random pattern");
                self.reset_to_random(memory);
                true  // Request a generation step after reset
            }
            _ => {
                // Unknown key
                println!("Unknown key pressed: 0x{:02X} ('{}')", key_code, key_code as char);
                false  // No action needed
            }
        }
    }

}

pub fn get_gol_palette() -> Vec<u8> {
    // Set up a colorful palette for Game of Life based on neighbor density
    // Color 0: Black (dead cells)
    // Colors 1-8: Different colors for live cells based on neighbor count
    // Colors 9-15: Extra colors for visual variety
    vec![
        // 0: Black (dead cells)
        0x00, 0x00, 0x00,
        // 1: Bright Blue (1 neighbor - very lonely)
        0x00, 0x00, 0xFF,
        // 2: Green (2 neighbors - stable, survival)
        0x00, 0xFF, 0x00,
        // 3: Bright Cyan (3 neighbors - birth/survival, optimal)
        0x00, 0xFF, 0xFF,
        // 4: Yellow (4 neighbors - getting crowded)
        0xFF, 0xFF, 0x00,
        // 5: Orange (5 neighbors - quite crowded)
        0xFF, 0x80, 0x00,
        // 6: Red (6 neighbors - very crowded)
        0xFF, 0x00, 0x00,
        // 7: Purple (7 neighbors - extremely crowded)
        0x80, 0x00, 0xFF,
        // 8: Bright Magenta (8 neighbors - maximum crowding)
        0xFF, 0x00, 0xFF,
        // 9-15: Extra gradient colors for visual effects
        // 9: Dark Green
        0x00, 0x80, 0x00,
        // 10: Pink
        0xFF, 0x80, 0x80,
        // 11: Light Green
        0x80, 0xFF, 0x80,
        // 12: Light Blue
        0x80, 0x80, 0xFF,
        // 13: Light Yellow
        0xFF, 0xFF, 0x80,
        // 14: Gray
        0x80, 0x80, 0x80,
        // 15: White
        0xFF, 0xFF, 0xFF,
    ]
} 