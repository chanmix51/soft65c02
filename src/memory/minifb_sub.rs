use super::*;
use minifb::{InputCallback, Window, Scale, ScaleMode, WindowOptions};
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicU8, Ordering};
use std::sync::{Arc, mpsc};
use std::{thread, time};
use std::collections::HashMap;

pub const MINIFB_WIDTH: usize = 128;
pub const MINIFB_HEIGHT: usize = 96;
const KEY_STACK_POINTER: usize = 0x0030;

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
    byte:       AtomicU8,
}

pub struct MiniFBMemory {
    token:          Arc<CommunicationToken>,
    buffer:         Vec<u8>,
}

impl MiniFBMemory {
    pub fn new(kb: Option<mpsc::Sender<u32>>) -> MiniFBMemory {
        let token = Arc::new( CommunicationToken {
            is_calling: AtomicBool::new(false),
            address: AtomicUsize::new(0),
            byte: AtomicU8::new(0),
        });
        let rtoken = token.clone();

        thread::spawn(move || {
            let mut window = Window::new(
                "65C02 computer graphic",
                MINIFB_WIDTH,
                MINIFB_HEIGHT,
                WindowOptions {
                    resize: true,
                    scale: Scale::FitScreen,
                    scale_mode: ScaleMode::AspectRatioStretch,
                    ..WindowOptions::default()
                },
            )
            .expect("Failed to open window.");

            if let Some(tx) = kb {
                window.set_input_callback(Box::new(KeyboardBuffer { sender: tx }));
            }
            // Limit to max ~60 fps update rate
            window.limit_update_rate(Some(time::Duration::from_micros(16600)));
            let mut memory:Vec<u32> = vec![0; MINIFB_WIDTH * MINIFB_HEIGHT];
            let mut palette:HashMap<u8, (u8, u8, u8)> = HashMap::new();
            palette.insert(0, (0x00, 0x00, 0x00));
            palette.insert(1, (0x88, 0x00, 0x00));
            palette.insert(2, (0x00, 0x88, 0x00));
            palette.insert(3, (0x00, 0x00, 0x88));
            palette.insert(4, (0x88, 0x88, 0x00));
            palette.insert(5, (0x88, 0x00, 0x88));
            palette.insert(6, (0x00, 0x88, 0x88));
            palette.insert(7, (0x88, 0x88, 0x88));
            palette.insert(8, (0x22, 0x22, 0x22));
            palette.insert(9, (0xff, 0x00, 0x00));
            palette.insert(10, (0x00, 0xff, 0x00));
            palette.insert(11, (0x00, 0x00, 0xff));
            palette.insert(12, (0xff, 0xff, 0x00));
            palette.insert(13, (0xff, 0x00, 0xff));
            palette.insert(14, (0x00, 0xff, 0xff));
            palette.insert(15, (0xff, 0xff, 0xff));
            let palette = palette;

            loop {
                if rtoken.is_calling.load(Ordering::Acquire) {
                    let byte = rtoken.byte.load(Ordering::SeqCst);
                    let addr = rtoken.address.load(Ordering::SeqCst) * 2;
                    let (loval, hival) = (byte & 0x0F, byte >> 4);
                    let byte1:u32 = {
                        let (r, g, b) = palette.get(&loval).expect("palette overflow ?");
                        (*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)
                    };
                    let byte2:u32 = {
                        let (r, g, b) = palette.get(&hival).expect("palette overflow ?");
                        (*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)
                    };
                    memory[addr] = byte1;
                    memory[addr + 1] = byte2;
                    rtoken.is_calling.store(false, Ordering::SeqCst);
                } 
                window
                    .update_with_buffer(&(memory), MINIFB_WIDTH, MINIFB_HEIGHT)
                    .unwrap();
                thread::sleep(time::Duration::from_micros(100))
            }
        });

        MiniFBMemory {
            token:          token,
            buffer:         vec![0; MINIFB_WIDTH * MINIFB_HEIGHT / 2],
        }
    }
}

impl AddressableIO for MiniFBMemory {
    fn get_size(&self) -> usize {
        MINIFB_WIDTH * MINIFB_HEIGHT / 2
    }

    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if self.buffer.len() >= addr + len {
            Ok(self.buffer[addr..addr + len].to_vec())
        } else {
            Err(MemoryError::ReadOverflow(len, addr, self.buffer.len()))
        }
    }

    fn write(&mut self, addr: usize, data: &Vec<u8>) -> Result<(), MemoryError> {
        for (offset, byte) in data.iter().enumerate() {
            self.buffer[addr + offset] = *byte;
            while self.token.is_calling.load(Ordering::Acquire) {
                thread::sleep(time::Duration::from_micros(10))
            }
            self.token.is_calling.store(true, Ordering::Release);
            self.token.address.store(addr + offset, Ordering::SeqCst);
            self.token.byte.store(*byte, Ordering::SeqCst);
        }
        Ok(())
    }
}
