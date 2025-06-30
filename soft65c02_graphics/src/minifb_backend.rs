/*
 * MiniFB Display Backend for Soft65C02
 *
 * The framebuffer memory subsystem is composed as following:
 * #0x0000 → #0x002F    palette (16 × 3 bytes for RGB)
 * #0x0030 → #0x003F    keyboard keys pressed
 * #0x0040 → #0x00FF    unused¹
 * #0x0100 → #0x1900    video buffer
 *
 * ¹ Technically this is still RAM so it can be used to just store data. Be aware that it will
 * trigger token inspection on write hence might be less performant than a RAM memory subsystem.
 */
use soft65c02_lib::{AddressableIO, DisplayBackend, memory::MemoryError};
use minifb::{InputCallback, Key, Scale, ScaleMode, Window, WindowOptions};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::{thread, time};

pub const DISPLAY_WIDTH: usize = 128;
pub const DISPLAY_HEIGHT: usize = 96;
pub const BUFFER_VIDEO_START_ADDR: usize = 256;

struct InterruptHandler {
    sender: mpsc::Sender<u32>,
}

impl InputCallback for InterruptHandler {
    fn add_char(&mut self, uni_char: u32) {
        self.sender.send(uni_char).unwrap();
    }
}

pub struct CommunicationToken {
    is_calling: AtomicBool,
    address: AtomicUsize,
    len: AtomicUsize,
    is_active: AtomicBool,
}

pub struct MiniFBDisplay {
    token: Arc<CommunicationToken>,
    buffer: Arc<Mutex<Vec<u8>>>,
    input_receiver: Option<mpsc::Receiver<u32>>,
}

#[allow(dead_code)]
fn get_key_code(key: Key) -> u8 {
    match key {
        Key::Key1 => 0x01,
        Key::Key2 => 0x02,
        Key::Key3 => 0x03,
        Key::Key4 => 0x04,
        Key::Key5 => 0x05,
        Key::Key6 => 0x06,
        Key::Key7 => 0x07,
        Key::Key8 => 0x08,
        Key::Key9 => 0x09,
        Key::Key0 => 0x0a,
        Key::A => 0x0b,
        Key::B => 0x0c,
        Key::C => 0x0d,
        Key::D => 0x0e,
        Key::E => 0x0f,
        Key::F => 0x10,
        Key::G => 0x11,
        Key::H => 0x12,
        Key::I => 0x13,
        Key::J => 0x14,
        Key::K => 0x15,
        Key::L => 0x16,
        Key::M => 0x17,
        Key::N => 0x18,
        Key::O => 0x19,
        Key::P => 0x1a,
        Key::Q => 0x1b,
        Key::R => 0x1c,
        Key::S => 0x1d,
        Key::T => 0x1e,
        Key::U => 0x1f,
        Key::V => 0x20,
        Key::W => 0x21,
        Key::X => 0x22,
        Key::Y => 0x23,
        Key::Z => 0x24,
        Key::F1 => 0x25,
        Key::F2 => 0x26,
        Key::F3 => 0x27,
        Key::F4 => 0x28,
        Key::F5 => 0x29,
        Key::F6 => 0x2a,
        Key::F7 => 0x2b,
        Key::F8 => 0x2c,
        Key::F9 => 0x2d,
        Key::F10 => 0x2e,
        Key::F11 => 0x2f,
        Key::F12 => 0x30,
        Key::F13 => 0x31,
        Key::F14 => 0x32,
        Key::F15 => 0x33,
        Key::Down => 0x34,
        Key::Left => 0x35,
        Key::Right => 0x36,
        Key::Up => 0x37,
        Key::Apostrophe => 0x38,
        Key::Backquote => 0x39,
        Key::Backslash => 0x3a,
        Key::Comma => 0x3b,
        Key::Equal => 0x3c,
        Key::LeftBracket => 0x3d,
        Key::Minus => 0x3e,
        Key::Period => 0x3f,
        Key::RightBracket => 0x40,
        Key::Semicolon => 0x41,
        Key::Slash => 0x42,
        Key::Backspace => 0x43,
        Key::Delete => 0x44,
        Key::End => 0x45,
        Key::Enter => 0x46,
        Key::Escape => 0x47,
        Key::Home => 0x48,
        Key::Insert => 0x49,
        Key::Menu => 0x4a,
        Key::PageDown => 0x4b,
        Key::PageUp => 0x4c,
        Key::Pause => 0x4d,
        Key::Space => 0x4e,
        Key::Tab => 0x4f,
        Key::NumLock => 0x50,
        Key::CapsLock => 0x51,
        Key::ScrollLock => 0x52,
        Key::LeftShift => 0x53,
        Key::RightShift => 0x54,
        Key::LeftCtrl => 0x55,
        Key::RightCtrl => 0x56,
        Key::NumPad0 => 0x57,
        Key::NumPad1 => 0x58,
        Key::NumPad2 => 0x59,
        Key::NumPad3 => 0x5a,
        Key::NumPad4 => 0x5b,
        Key::NumPad5 => 0x5c,
        Key::NumPad6 => 0x5d,
        Key::NumPad7 => 0x5e,
        Key::NumPad8 => 0x5f,
        Key::NumPad9 => 0x60,
        Key::NumPadDot => 0x61,
        Key::NumPadSlash => 0x62,
        Key::NumPadAsterisk => 0x63,
        Key::NumPadMinus => 0x64,
        Key::NumPadPlus => 0x65,
        Key::NumPadEnter => 0x66,
        Key::LeftAlt => 0x67,
        Key::RightAlt => 0x68,
        Key::LeftSuper => 0x69,
        Key::RightSuper => 0x6a,
        Key::Unknown => 0x6b,
        Key::Count => 0x6c,
    }
}

impl MiniFBDisplay {
    pub fn new() -> MiniFBDisplay {
        let buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![
            0;
            DISPLAY_WIDTH * DISPLAY_HEIGHT / 2 + BUFFER_VIDEO_START_ADDR
        ]));
        let token = Arc::new(CommunicationToken {
            is_calling: AtomicBool::new(false),
            address: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
            is_active: AtomicBool::new(true),
        });
        
        let (input_tx, input_rx) = mpsc::channel();
        let rtoken = token.clone();
        let rbuffer = buffer.clone();

        thread::spawn(move || {
            let mut window = Window::new(
                "Soft-65C02 Display",
                DISPLAY_WIDTH,
                DISPLAY_HEIGHT,
                WindowOptions {
                    resize: true,
                    scale: Scale::X4,
                    scale_mode: ScaleMode::AspectRatioStretch,
                    ..WindowOptions::default()
                },
            )
            .expect("Failed to open window.");

            window.set_input_callback(Box::new(InterruptHandler { sender: input_tx }));
            
            let mut memory: Vec<u32> = vec![0; DISPLAY_WIDTH * DISPLAY_HEIGHT];
            
            loop {
                if !window.is_open() {
                    rtoken.is_active.store(false, Ordering::SeqCst);
                    break;
                }
                
                if rtoken.is_calling.load(Ordering::Acquire) {
                    let addr = rtoken.address.load(Ordering::SeqCst);
                    let len = rtoken.len.load(Ordering::SeqCst);
                    let buffer = rbuffer.lock().unwrap();

                    for (index, byte) in buffer[addr..addr + len].iter().enumerate() {
                        if addr + index >= BUFFER_VIDEO_START_ADDR {
                            let (loval, hival) = (byte & 0x0F, byte >> 4);
                            let byte1: u32 = {
                                let rgb = buffer
                                    .chunks(3)
                                    .nth(loval as usize)
                                    .expect("Where is the palette?");
                                (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | (rgb[2] as u32)
                            };
                            let byte2: u32 = {
                                let rgb = buffer
                                    .chunks(3)
                                    .nth(hival as usize)
                                    .expect("Where is the palette?");
                                (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | (rgb[2] as u32)
                            };
                            memory[(addr + index - BUFFER_VIDEO_START_ADDR) * 2] = byte1;
                            memory[(addr + index - BUFFER_VIDEO_START_ADDR) * 2 + 1] = byte2;
                        }
                    }
                    rtoken.is_calling.store(false, Ordering::SeqCst);
                }
                
                window
                    .update_with_buffer(&memory, DISPLAY_WIDTH, DISPLAY_HEIGHT)
                    .unwrap();
                    
                thread::sleep(time::Duration::from_micros(100));
            }
        });

        MiniFBDisplay { 
            token, 
            buffer,
            input_receiver: Some(input_rx),
        }
    }
}

impl AddressableIO for MiniFBDisplay {
    fn get_size(&self) -> usize {
        DISPLAY_WIDTH * DISPLAY_HEIGHT / 2 + BUFFER_VIDEO_START_ADDR
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
            buffer[addr + offset] = *byte;
        }
        self.token.is_calling.store(true, Ordering::Release);
        self.token.address.store(addr, Ordering::SeqCst);
        self.token.len.store(data.len(), Ordering::SeqCst);
        Ok(())
    }
}

impl DisplayBackend for MiniFBDisplay {
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