/*
 * Interactive Mandelbrot Set Explorer
 * 
 * Features:
 * - Real-time fractal exploration with parameter adjustment
 * - Beautiful gradient palette (blue→purple→red→orange→yellow→white)
 * - Interactive navigation: pan, zoom, iteration control
 * - International keyboard layout compatibility
 * - High-performance computation optimized for 128×96 display
 * 
 * Controls:
 * - Arrow keys: Pan view in complex plane
 * - +/- keys: Zoom in/out (works on all keyboard layouts)
 * - I/D keys: Increase/decrease iteration limit for detail vs speed
 * - R key: Reset to default view (-0.5+0i, zoom 2.5, 32 iterations)
 * 
 * Technical Notes:
 * - Uses hybrid keyboard input for international compatibility
 * - Caches computation results to avoid unnecessary recalculation
 * - Maps iteration count to 16-color palette for visual appeal
 * - Handles edge cases like maximum iteration limits gracefully
 */

use soft65c02_lib::{Memory, AddressableIO};

// ASCII key codes from ReceivedCharacter events (layout-aware)
const KEY_R: u8 = b'R';      // R key for reset (ASCII 'R')
const KEY_I: u8 = b'I';      // I key - increase iterations (ASCII 'I')
const KEY_D: u8 = b'D';      // D key - decrease iterations (ASCII 'D')

// Navigation key codes (special codes from get_special_key_code - layout-independent)
const KEY_UP: u8 = 0x80;     // Up arrow
const KEY_DOWN: u8 = 0x81;   // Down arrow
const KEY_LEFT: u8 = 0x82;   // Left arrow
const KEY_RIGHT: u8 = 0x83;  // Right arrow

// Symbol character codes (from ReceivedCharacter events - ASCII values, layout-aware)
const CHAR_PLUS: u8 = b'+';     // '+' character
const CHAR_MINUS: u8 = b'-';    // '-' character
const CHAR_EQUALS: u8 = b'=';   // '=' character

// Constants for Mandelbrot computation
const SCREEN_WIDTH: usize = 128;
const SCREEN_HEIGHT: usize = 96;
const BYTES_PER_ROW: usize = SCREEN_WIDTH / 2;  // 2 pixels per byte
const VIDEO_BUFFER_START: usize = 0x8100;

// Default Mandelbrot view parameters
const DEFAULT_CENTER_X: f64 = -0.5;
const DEFAULT_CENTER_Y: f64 = 0.0;
const DEFAULT_ZOOM: f64 = 2.5;
const DEFAULT_MAX_ITERATIONS: u32 = 32;

// Zoom and pan factors
const ZOOM_FACTOR: f64 = 1.5;
const PAN_FACTOR: f64 = 0.1;

pub struct MandelbrotState {
    center_x: f64,
    center_y: f64,
    zoom: f64,
    max_iterations: u32,
    frame_buffer: Vec<u8>,  // Cached computation result
    write_buffer: Vec<u8>,  // Reusable buffer for memory writes
    needs_recompute: bool,  // Flag to track if we need to recompute
}

impl MandelbrotState {
    pub fn new() -> Self {
        Self {
            center_x: DEFAULT_CENTER_X,
            center_y: DEFAULT_CENTER_Y,
            zoom: DEFAULT_ZOOM,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            frame_buffer: vec![0u8; SCREEN_WIDTH * SCREEN_HEIGHT],
            write_buffer: vec![0u8; BYTES_PER_ROW * SCREEN_HEIGHT],
            needs_recompute: true,
        }
    }
    
    pub fn reset_to_default(&mut self, memory: &mut Memory) {
        println!("🎨 Resetting Mandelbrot to default view...");
        self.center_x = DEFAULT_CENTER_X;
        self.center_y = DEFAULT_CENTER_Y;
        self.zoom = DEFAULT_ZOOM;
        self.max_iterations = DEFAULT_MAX_ITERATIONS;
        self.needs_recompute = true;
        
        self.compute_mandelbrot();
        self.write_to_memory(memory);
        
        println!("Reset to center=({:.6}, {:.6}), zoom={:.6}, iterations={}", 
                 self.center_x, self.center_y, self.zoom, self.max_iterations);
    }
    
    pub fn compute_next_generation(&mut self) {
        // For Mandelbrot, "next generation" means recompute if parameters changed
        if self.needs_recompute {
            self.compute_mandelbrot();
            self.needs_recompute = false;
        }
    }
    
    fn compute_mandelbrot(&mut self) {
        println!("🎨 Computing Mandelbrot set...");
        let start_time = std::time::Instant::now();
        
        // Calculate the complex plane bounds
        let width = self.zoom;
        let height = self.zoom * (SCREEN_HEIGHT as f64 / SCREEN_WIDTH as f64);
        
        let min_x = self.center_x - width / 2.0;
        let max_x = self.center_x + width / 2.0;
        let min_y = self.center_y - height / 2.0;
        let max_y = self.center_y + height / 2.0;
        
        let dx = (max_x - min_x) / SCREEN_WIDTH as f64;
        let dy = (max_y - min_y) / SCREEN_HEIGHT as f64;
        
        // Compute Mandelbrot set for each pixel
        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let cx = min_x + x as f64 * dx;
                let cy = min_y + y as f64 * dy;
                
                let iterations = self.mandelbrot_iterations(cx, cy);
                
                // Map iterations to color index (0-15 for 4-bit color)
                let color_index = if iterations == self.max_iterations {
                    0  // Black for points in the set
                } else {
                    // Map iterations to colors 1-15
                    ((iterations * 14) / self.max_iterations + 1).min(15) as u8
                };
                
                self.frame_buffer[y * SCREEN_WIDTH + x] = color_index;
            }
        }
        
        let elapsed = start_time.elapsed();
        println!("Mandelbrot computation completed in {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    }
    
    #[inline]
    fn mandelbrot_iterations(&self, cx: f64, cy: f64) -> u32 {
        let mut zx = 0.0;
        let mut zy = 0.0;
        let mut iterations = 0;
        
        while iterations < self.max_iterations {
            let zx_sq = zx * zx;
            let zy_sq = zy * zy;
            
            // Check for divergence (|z|² > 4)
            if zx_sq + zy_sq > 4.0 {
                break;
            }
            
            // z = z² + c
            let new_zx = zx_sq - zy_sq + cx;
            let new_zy = 2.0 * zx * zy + cy;
            
            zx = new_zx;
            zy = new_zy;
            iterations += 1;
        }
        
        iterations
    }
    
    pub fn write_to_memory(&mut self, memory: &mut Memory) {
        // Clear reusable buffer
        self.write_buffer.fill(0);
        
        // Pack pixels into nibbles (2 pixels per byte)
        for y in 0..SCREEN_HEIGHT {
            let row_offset = y * BYTES_PER_ROW;
            for x in 0..SCREEN_WIDTH {
                let byte_index = row_offset + (x / 2);
                let is_upper_nibble = (x % 2) == 1;
                let pixel_value = self.frame_buffer[y * SCREEN_WIDTH + x];
                
                if byte_index < self.write_buffer.len() {
                    if is_upper_nibble {
                        self.write_buffer[byte_index] |= pixel_value << 4;
                    } else {
                        self.write_buffer[byte_index] |= pixel_value;
                    }
                }
            }
        }
        
        // Write to video buffer
        memory.write(VIDEO_BUFFER_START, &self.write_buffer).unwrap();
    }
    
    pub fn process_keyboard_input(&mut self, key_code: u8, memory: &mut Memory) -> bool {
        // Handle all input using simplified approach
        let needs_update = match key_code {
            // Navigation (layout-independent arrows)
            KEY_UP => {
                self.center_y -= self.zoom * PAN_FACTOR;
                println!("⬆️  Panning up to ({:.6}, {:.6})", self.center_x, self.center_y);
                true
            }
            KEY_DOWN => {
                self.center_y += self.zoom * PAN_FACTOR;
                println!("⬇️  Panning down to ({:.6}, {:.6})", self.center_x, self.center_y);
                true
            }
            KEY_LEFT => {
                self.center_x -= self.zoom * PAN_FACTOR;
                println!("⬅️  Panning left to ({:.6}, {:.6})", self.center_x, self.center_y);
                true
            }
            KEY_RIGHT => {
                self.center_x += self.zoom * PAN_FACTOR;
                println!("➡️  Panning right to ({:.6}, {:.6})", self.center_x, self.center_y);
                true
            }
            
            // Zoom controls (layout-aware characters from ReceivedCharacter events)
            CHAR_PLUS | CHAR_EQUALS => {
                self.zoom /= ZOOM_FACTOR;
                let symbol = if key_code == CHAR_PLUS { "+" } else { "=" };
                println!("🔍 Zooming in ({}), new zoom level: {:.6}", symbol, self.zoom);
                true
            }
            CHAR_MINUS => {
                self.zoom *= ZOOM_FACTOR;
                println!("🔍 Zooming out (-), new zoom level: {:.6}", self.zoom);
                true
            }
            
            // Command keys - accept both upper and lower case
            KEY_R | b'r' => {
                println!("🎨 Reset key pressed - returning to default view");
                self.reset_to_default(memory);
                return true;  // Already updated memory
            }
            KEY_I | b'i' => {
                self.max_iterations = (self.max_iterations + 8).min(256);
                println!("🔄 Increased iterations to {}", self.max_iterations);
                true
            }
            KEY_D | b'd' => {
                self.max_iterations = (self.max_iterations.saturating_sub(8)).max(8);
                println!("🔄 Decreased iterations to {}", self.max_iterations);
                true
            }
            
            _ => {
                println!("Unknown key pressed: 0x{:02X} ('{}')", key_code, key_code as char);
                return false;
            }
        };
        
        if needs_update {
            self.needs_recompute = true;
        }
        
        needs_update
    }
}

pub fn get_mandelbrot_palette() -> Vec<u8> {
    // Create a beautiful gradient palette for Mandelbrot visualization
    // Color 0: Black (points in the set)
    // Colors 1-15: Gradient from blue through purple, red, orange, yellow to white
    vec![
        // 0: Black (in the set)
        0x00, 0x00, 0x00,
        // 1: Deep Blue
        0x00, 0x00, 0x80,
        // 2: Blue
        0x00, 0x00, 0xFF,
        // 3: Blue-Purple
        0x40, 0x00, 0xFF,
        // 4: Purple
        0x80, 0x00, 0xFF,
        // 5: Purple-Red
        0xC0, 0x00, 0xFF,
        // 6: Magenta
        0xFF, 0x00, 0xFF,
        // 7: Red-Pink
        0xFF, 0x00, 0x80,
        // 8: Red
        0xFF, 0x00, 0x00,
        // 9: Orange-Red
        0xFF, 0x40, 0x00,
        // 10: Orange
        0xFF, 0x80, 0x00,
        // 11: Yellow-Orange
        0xFF, 0xC0, 0x00,
        // 12: Yellow
        0xFF, 0xFF, 0x00,
        // 13: Light Yellow
        0xFF, 0xFF, 0x80,
        // 14: Near White
        0xFF, 0xFF, 0xC0,
        // 15: White (fastest divergence)
        0xFF, 0xFF, 0xFF,
    ]
} 