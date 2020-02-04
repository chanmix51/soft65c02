use super::*;
use minifb::{InputCallback, Window, Scale, ScaleMode, WindowOptions};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc, Mutex};
use std::{thread, time};
use std::collections::HashMap;

pub const MINIFB_WIDTH: usize = 128;
pub const MINIFB_HEIGHT: usize = 96;

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
        let buffer = Arc::new(Mutex::new(vec![0; MINIFB_WIDTH * MINIFB_HEIGHT / 2]));
        let token = Arc::new( CommunicationToken {
            is_calling: AtomicBool::new(false),
            address: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
        });
        let rtoken = token.clone();
        let rbuffer = buffer.clone();

        thread::spawn(move || {
            let mut window = Window::new(
                "65C02 computer graphic",
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
                    let addr = rtoken.address.load(Ordering::SeqCst);
                    let len  = rtoken.len.load(Ordering::SeqCst);
                    let buffer = rbuffer.lock().unwrap();

                    for (index, byte) in buffer[addr..addr + len].iter().enumerate() {
                        let (loval, hival) = (byte & 0x0F, byte >> 4);
                        let byte1:u32 = {
                            let (r, g, b) = palette.get(&loval).expect("palette overflow ?");
                            (*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)
                        };
                        let byte2:u32 = {
                            let (r, g, b) = palette.get(&hival).expect("palette overflow ?");
                            (*r as u32) << 16 | (*g as u32) << 8 | (*b as u32)
                        };
                        memory[addr + index * 2] = byte1;
                        memory[addr + index * 2 + 1] = byte2;
                    }
                    rtoken.is_calling.store(false, Ordering::SeqCst);
                } else {
                    window
                        .update_with_buffer(&(memory), MINIFB_WIDTH, MINIFB_HEIGHT)
                        .unwrap();
                }
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
        MINIFB_WIDTH * MINIFB_HEIGHT / 2
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
