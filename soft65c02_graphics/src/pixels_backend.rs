/*
 * Pixels Display Backend for Soft65C02
 *
 * Modern graphics backend using pixels + winit for better cross-platform support,
 * particularly improved Wayland compatibility.
 *
 * Uses hybrid keyboard input system for international layout compatibility:
 * - ReceivedCharacter events for all printable characters (numbers, letters, symbols)
 * - KeyboardInput events only for special keys (arrows, escape, enter)
 *
 * Internal memory layout (relative to subsystem start):
 * #0x0000 → #0x002F    palette (16 × 3 bytes for RGB)
 * #0x0030              keyboard input buffer (single key code byte)
 * #0x0031 → #0x003F    unused
 * #0x0040 → #0x00FF    unused¹
 * #0x0100 → #0x1900    video buffer (128×96 pixels, 4-bit color, 2 pixels per byte)
 *
 * ¹ Technically this is still RAM so it can be used to just store data. Be aware that it will
 * trigger token inspection on write hence might be less performant than a RAM memory subsystem.
 *
 * Keyboard Layout Compatibility:
 * - All printable characters handled by ReceivedCharacter (layout-aware)
 * - Special keys (arrows, function keys) handled by KeyboardInput (layout-independent)
 * - Applications receive proper ASCII values for all character input
 */
use soft65c02_lib::{AddressableIO, DisplayBackend, memory::MemoryError};
use pixels::{Pixels, SurfaceTexture};
use winit::{
    dpi::LogicalSize,
    event::{Event, WindowEvent, ElementState, VirtualKeyCode, KeyboardInput},
    event_loop::ControlFlow,
    window::{Window, WindowBuilder},
};

// Import EventLoop and EventLoopBuilder conditionally
#[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
use winit::event_loop::EventLoopBuilder;

#[cfg(not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
use winit::event_loop::EventLoop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

// Platform-specific imports for Linux
#[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
use winit::platform::x11::EventLoopBuilderExtX11;

pub const DISPLAY_WIDTH: usize = 128;
pub const DISPLAY_HEIGHT: usize = 96;
pub const BUFFER_VIDEO_START_ADDR: usize = 0x0100;
pub const TOTAL_MEMORY_SIZE: usize = 0x1900;  // Up to end of video buffer

// Keyboard buffer layout - export for game modules
pub const KEYBOARD_KEY_ADDR: usize = 0x30;       // Key code

pub struct CommunicationToken {
    is_calling: AtomicBool,
    is_active: AtomicBool,
}

struct WindowState {
    pixels: Pixels,
    token: Arc<CommunicationToken>,
    buffer: Arc<Mutex<Vec<u8>>>,
    input_tx: mpsc::Sender<u32>,
}

impl WindowState {
    fn new(
        window: &Window,
        token: Arc<CommunicationToken>,
        buffer: Arc<Mutex<Vec<u8>>>,
        input_tx: mpsc::Sender<u32>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, window);
        let mut pixels = Pixels::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32, surface_texture)?;
        
        // Initialize with black background
        Self::clear_frame(pixels.frame_mut());
        pixels.render()?;
        
        Ok(Self {
            pixels,
            token,
            buffer,
            input_tx,
        })
    }
    
    fn clear_frame(frame: &mut [u8]) {
        for pixel in frame.chunks_exact_mut(4) {
            pixel[0] = 0;   // R
            pixel[1] = 0;   // G  
            pixel[2] = 0;   // B
            pixel[3] = 255; // A
        }
    }
    
    fn handle_window_event(&mut self, event: WindowEvent, control_flow: &mut ControlFlow) {
        match event {
            WindowEvent::CloseRequested => {
                self.token.is_active.store(false, Ordering::SeqCst);
                *control_flow = ControlFlow::Exit;
            }
            // Handle character input (layout-aware) - for symbols like +, -, =, etc.
            WindowEvent::ReceivedCharacter(ch) => {
                // Pass through ASCII characters exactly as received - let receivers decide case handling
                if ch.is_ascii() && !ch.is_control() {
                    let code = ch as u8;
                    let _ = self.input_tx.send(code as u32);
                    self.write_to_keyboard_buffer(code);
                }
            }
            // Handle special keys (layout-independent) - arrows, numbers, letters for commands
            WindowEvent::KeyboardInput {
                input: KeyboardInput {
                    virtual_keycode: Some(key_code),
                    state: ElementState::Pressed,
                    ..
                },
                ..
            } => {
                if let Some(code) = get_special_key_code(key_code) {
                    let _ = self.input_tx.send(code as u32);
                    self.write_to_keyboard_buffer(code);
                }
            }
            WindowEvent::Resized(size) => {
                if let Err(err) = self.pixels.resize_surface(size.width, size.height) {
                    eprintln!("pixels.resize_surface() failed: {err}");
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    }

    fn write_to_keyboard_buffer(&self, code: u8) {
        let mut buffer = self.buffer.lock().unwrap();
        if buffer.len() > KEYBOARD_KEY_ADDR {
            buffer[KEYBOARD_KEY_ADDR] = code;
        }
    }
    
    fn update_display(&mut self) {
        if !self.token.is_calling.load(Ordering::Acquire) {
            return;
        }
        
        let buffer = self.buffer.lock().unwrap();
        let frame = self.pixels.frame_mut();
        
        Self::render_video_buffer(&buffer, frame);
        self.token.is_calling.store(false, Ordering::SeqCst);
    }
    
    fn render_video_buffer(buffer: &[u8], frame: &mut [u8]) {
        // Always render the entire video buffer
        let video_buffer_start = BUFFER_VIDEO_START_ADDR;
        let video_buffer_size = (DISPLAY_WIDTH / 2) * DISPLAY_HEIGHT; // 2 pixels per byte
        
        for video_offset in 0..video_buffer_size {
            let buffer_index = video_buffer_start + video_offset;
            if buffer_index >= buffer.len() {
                break;
            }
            
            let byte = buffer[buffer_index];
            let pixel_x = (video_offset % (DISPLAY_WIDTH / 2)) * 2; // 64 bytes per row, 2 pixels per byte
            let pixel_y = video_offset / (DISPLAY_WIDTH / 2);
            
            if pixel_y >= DISPLAY_HEIGHT || pixel_x >= DISPLAY_WIDTH - 1 {
                continue;
            }
            
            let (loval, hival) = (byte & 0x0F, byte >> 4);
            
            // Render left pixel (lower nibble)
            Self::render_pixel(buffer, frame, loval, pixel_x, pixel_y);
            
            // Render right pixel (upper nibble)  
            Self::render_pixel(buffer, frame, hival, pixel_x + 1, pixel_y);
        }
    }
    
    fn render_pixel(buffer: &[u8], frame: &mut [u8], color_index: u8, x: usize, y: usize) {
        if color_index >= 16 {
            return;
        }
        
        let rgba_offset = (y * DISPLAY_WIDTH + x) * 4;
        if rgba_offset + 3 >= frame.len() {
            return;
        }
        
        // Read palette colors from the start of memory (0x0000) where the palette is stored
        frame[rgba_offset] = buffer[(color_index as usize) * 3];     // R
        frame[rgba_offset + 1] = buffer[(color_index as usize) * 3 + 1]; // G
        frame[rgba_offset + 2] = buffer[(color_index as usize) * 3 + 2]; // B
        frame[rgba_offset + 3] = 255; // A
    }
    
    fn render(&mut self) -> Result<(), pixels::Error> {
        self.pixels.render()
    }
}

pub struct PixelsDisplay {
    token: Arc<CommunicationToken>,
    buffer: Arc<Mutex<Vec<u8>>>,
    input_receiver: Option<mpsc::Receiver<u32>>,
}

fn create_event_loop() -> Result<winit::event_loop::EventLoop<()>, Box<dyn std::error::Error>> {
    #[cfg(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd"))]
    {
        Ok(EventLoopBuilder::new().with_any_thread(true).build())
    }
    #[cfg(not(any(target_os = "linux", target_os = "dragonfly", target_os = "freebsd", target_os = "netbsd", target_os = "openbsd")))]
    {
        Ok(EventLoop::new())
    }
}

fn create_window(event_loop: &winit::event_loop::EventLoop<()>) -> Result<Window, Box<dyn std::error::Error>> {
    let size = LogicalSize::new(DISPLAY_WIDTH as f64 * 4.0, DISPLAY_HEIGHT as f64 * 4.0);
    let window = WindowBuilder::new()
        .with_title("Soft-65C02 Display (Pixels)")
        .with_inner_size(size)
        .with_min_inner_size(size)
        .build(event_loop)?;
    Ok(window)
}

fn run_event_loop(
    event_loop: winit::event_loop::EventLoop<()>,
    window: Window,
    token: Arc<CommunicationToken>,
    buffer: Arc<Mutex<Vec<u8>>>,
    input_tx: mpsc::Sender<u32>,
) {
    let mut window_state = match WindowState::new(&window, token, buffer, input_tx) {
        Ok(state) => state,
        Err(e) => {
            eprintln!("Failed to create window state: {}", e);
            return;
        }
    };
    
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::WindowEvent { event, .. } => {
                window_state.handle_window_event(event, control_flow);
            }
            Event::MainEventsCleared => {
                window_state.update_display();
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                if window_state.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}

// Map only truly special keys that don't produce characters
// All character input (numbers, letters, symbols) handled by ReceivedCharacter events
fn get_special_key_code(key: VirtualKeyCode) -> Option<u8> {
    match key {
        // Arrow keys (never produce characters)
        VirtualKeyCode::Up => Some(0x80),
        VirtualKeyCode::Down => Some(0x81),
        VirtualKeyCode::Left => Some(0x82),
        VirtualKeyCode::Right => Some(0x83),
        
        // System keys that don't produce printable characters
        VirtualKeyCode::Escape => Some(0x1B),
        VirtualKeyCode::Return => Some(0x0D),
        
        _ => None,  // Everything else handled by ReceivedCharacter
    }
}

impl PixelsDisplay {
    pub fn new() -> Self {
        let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![0; TOTAL_MEMORY_SIZE]));
        let token = Arc::new(CommunicationToken {
            is_calling: AtomicBool::new(false),
            is_active: AtomicBool::new(true),
        });
        
        let (input_tx, input_rx) = mpsc::channel();
        let rtoken = token.clone();
        let rbuffer = buffer.clone();

        thread::spawn(move || {
            let event_loop = match create_event_loop() {
                Ok(el) => el,
                Err(e) => {
                    eprintln!("Failed to create event loop: {}", e);
                    return;
                }
            };
            
            let window = match create_window(&event_loop) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Failed to create window: {}", e);
                    return;
                }
            };
            
            run_event_loop(event_loop, window, rtoken, rbuffer, input_tx);
        });

        Self { 
            token, 
            buffer,
            input_receiver: Some(input_rx),
        }
    }
}

impl AddressableIO for PixelsDisplay {
    fn get_size(&self) -> usize {
        TOTAL_MEMORY_SIZE
    }

    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        let buffer = self.buffer.lock().unwrap();
        if buffer.len() >= addr + len {
            Ok(buffer[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr))
        }
    }

    fn write(&mut self, addr: usize, data: &[u8]) -> Result<(), MemoryError> {
        let mut buffer = self.buffer.lock().unwrap();
        for (offset, byte) in data.iter().enumerate() {
            if addr + offset < buffer.len() {
                buffer[addr + offset] = *byte;
            }
        }
        self.token.is_calling.store(true, Ordering::Release);
        Ok(())
    }
}

impl DisplayBackend for PixelsDisplay {
    fn get_dimensions(&self) -> (usize, usize) {
        (DISPLAY_WIDTH, DISPLAY_HEIGHT)
    }
    
    fn is_active(&self) -> bool {
        self.token.is_active.load(Ordering::SeqCst)
    }
    
    fn get_input_events(&mut self) -> Vec<u32> {
        let mut events = Vec::new();
        if let Some(ref receiver) = self.input_receiver {
            while let Ok(event) = receiver.try_recv() {
                events.push(event);
            }
        }
        events
    }
} 