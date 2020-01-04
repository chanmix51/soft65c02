use std::sync::mpsc;
use minifb::{Window, InputCallback};
use super::*;

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

/*
 * MiniFBMemoryAdapter
 * This adapter maps the 6502 memory on the framebuffer memory, it acts as a
 * virtual video card.
 *
 * MEMORY ALLOCATION MAP
 * 0x0000 → 0x002F  palette
 * 0x0030 → 0x0033  keyboard memory, the last key pressed.
 * 0x0100 → 0x1900  video memory
 *
 * Each byte of the video memory is mapped to 2 pixels in the framebuffer
 * memory, 4 bits defining the index in the palette for the RGB values for each
 * pixel.
 * The 48 first bytes are used for the palette and must be set by the software
 * at startup. The key stack is updated by the minifb library.
 */
pub struct MiniFBMemoryAdapter {
    minifb:     Vec<u32>,
    memory:     Box<[u8; MINIFB_WIDTH * MINIFB_HEIGHT / 2 + 0xFF]>,
    window:     Window,
    receiver:   mpsc::Receiver<u32>,
}

impl MiniFBMemoryAdapter {
    pub fn new(mut window: Window) -> MiniFBMemoryAdapter {
        let (tx, rx) = mpsc::channel::<u32>();
        window.set_input_callback(Box::new(KeyboardBuffer { sender: tx }));

        MiniFBMemoryAdapter {
            minifb:     vec![0; MINIFB_WIDTH * MINIFB_HEIGHT],
            memory:     Box::new([0; MINIFB_WIDTH * MINIFB_HEIGHT / 2 + 0xFF]),
            window:     window,
            receiver:   rx,
        }
    }

    fn update_minifb(&mut self, addr: usize) {
        let minifb_addr = (addr - 0xFF) * 2;
        let byte = self.memory[addr];
        let lo_byte = byte & 0b00001111;
        let hi_byte = byte >> 4;
        let (r, g, b) = (
            self.memory[lo_byte as usize * 3],
            self.memory[lo_byte as usize * 3 + 1],
            self.memory[lo_byte as usize * 3 + 2]
            );
        self.minifb[minifb_addr] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        let (r, g, b) = (
            self.memory[hi_byte as usize * 3],
            self.memory[hi_byte as usize * 3 + 1],
            self.memory[hi_byte as usize * 3 + 2]
            );
        self.minifb[minifb_addr + 1] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
    }

    pub fn window_update(&mut self) -> Result<(), minifb::Error> {
        self.window.update_with_buffer(&(self.minifb), MINIFB_WIDTH, MINIFB_HEIGHT)
    }
}

impl AddressableIO for MiniFBMemoryAdapter {
    fn get_size(&self) -> usize {
        MINIFB_WIDTH * MINIFB_HEIGHT / 2 + 0xFF
    }

    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        // reading the keyboard memory
        if addr == KEY_STACK_POINTER && len == 1 {
            let byte = match self.receiver.try_recv() {
                Ok(v) => {
                    u32::to_ne_bytes(v)[0]
                },
                Err(e) => {
                    0x00
                    },
            };
            return Ok(vec![byte]);
        }

        Ok(self.memory[addr..addr + len].to_vec())
    }

    /*
     * TODO: handle overflows
     */
    fn write(&mut self, addr: usize, data: Vec<u8>) -> Result<(), MemoryError> {
        let mut offset = 0;
        for byte in data.iter() {
            let pointer = addr + offset;
            self.memory[pointer] = *byte;
            if pointer > 0xFF {
                self.update_minifb(pointer);
            }
            offset += 1;
        }
        self.window_update();
        Ok(())
    }
}
