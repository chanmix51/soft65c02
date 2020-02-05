/*
 * Soft65C02 Mini Framebuffer
 *
 * The framebuffer memory subsystem is composed as following:
 * #0x0000 → #0x002F    palette ( 16 × 3 bytes for RGB)
 * #0x0030              keyboard input
 * #0x0031 → #0x00FF    unused
 * #0x0100 → #0x1900    video buffer
 */
use super::*;
use minifb::{InputCallback, Window, Scale, ScaleMode, WindowOptions};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc, Mutex};
use std::{thread, time};

pub const MINIFB_WIDTH: usize = 128;
pub const MINIFB_HEIGHT: usize = 96;
pub const BUFFER_VIDEO_START_ADDR:usize = 256;

struct KeyboardBuffer {
    sender: mpsc::Sender<u32>,
}

impl InputCallback for KeyboardBuffer {
    fn add_char(&mut self, uni_char: u32) {
        self.sender.send(uni_char).unwrap();
    }
}

pub struct CommunicationToken {
    is_calling: AtomicBool,
    address:    AtomicUsize,
    len:        AtomicUsize
}

pub struct MiniFBMemory {
    token:          Arc<CommunicationToken>,
    buffer:         Arc<Mutex<Vec<u8>>>,
}

impl MiniFBMemory {
    pub fn new(kb: Option<mpsc::Sender<u32>>) -> MiniFBMemory {
        let buffer:Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(vec![0; MINIFB_WIDTH * MINIFB_HEIGHT / 2 + BUFFER_VIDEO_START_ADDR]));
        let token = Arc::new( CommunicationToken {
            is_calling: AtomicBool::new(false),
            address: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
        });
        let rtoken = token.clone();
        let rbuffer = buffer.clone();

        thread::spawn(move || {
            let mut window = Window::new(
                "Soft-65C02 Mini Framebuffer",
                MINIFB_WIDTH,
                MINIFB_HEIGHT,
                WindowOptions {
                    resize: true,
                    scale: Scale::X4,
                    scale_mode: ScaleMode::AspectRatioStretch,
                    ..WindowOptions::default()
                },
            )
            .expect("Failed to open window.");

            if let Some(tx) = kb {
                window.set_input_callback(Box::new(KeyboardBuffer { sender: tx }));
            }
            let mut memory:Vec<u32> = vec![0; MINIFB_WIDTH * MINIFB_HEIGHT];

            loop {
                if rtoken.is_calling.load(Ordering::Acquire) {
                    let addr = rtoken.address.load(Ordering::SeqCst);
                    let len  = rtoken.len.load(Ordering::SeqCst);
                    let buffer = rbuffer.lock().unwrap();

                    for (index, byte) in buffer[addr..addr + len].iter().enumerate() {
                        if addr + index >= BUFFER_VIDEO_START_ADDR {
                            let (loval, hival) = (byte & 0x0F, byte >> 4);
                            let byte1:u32 = {
                                let rgb = buffer.chunks(3).nth(loval as usize).expect("Where is the palette?");
                                (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | (rgb[2] as u32)
                            };
                            let byte2:u32 = {
                                let rgb = buffer.chunks(3).nth(hival as usize).expect("Where is the palette?");
                                (rgb[0] as u32) << 16 | (rgb[1] as u32) << 8 | (rgb[2] as u32)
                            };
                            memory[(addr + index - BUFFER_VIDEO_START_ADDR) * 2] = byte1;
                            memory[(addr + index - BUFFER_VIDEO_START_ADDR) * 2 + 1] = byte2;
                        }
                    }
                    rtoken.is_calling.store(false, Ordering::SeqCst);
                }
                window
                    .update_with_buffer(&(memory), MINIFB_WIDTH, MINIFB_HEIGHT)
                    .unwrap();
                thread::sleep(time::Duration::from_micros(10));

            }
        });

        MiniFBMemory {
            token:          token,
            buffer:         buffer,
        }
    }
}

impl AddressableIO for MiniFBMemory {
    fn get_size(&self) -> usize {
        MINIFB_WIDTH * MINIFB_HEIGHT / 2 + BUFFER_VIDEO_START_ADDR
    }

    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        let buffer = self.buffer.lock().unwrap();
        if buffer.len() >= addr + len {
            Ok(buffer[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr, buffer.len()))
        }
    }

    fn write(&mut self, addr: usize, data: &Vec<u8>) -> Result<(), MemoryError> {
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
