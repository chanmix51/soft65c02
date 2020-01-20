/*
 * MiniFBMemoryAdapter
 * This adapter maps the 6502 memory on the framebuffer memory, it acts as a
 * virtual video card.
 *
 * MEMORY ALLOCATION MAP
 * 0x0000 → 0x002F  RGB palette (16 colors)
 * 0x0030           keyboard register (ro)
 * 0x0031           scrolling register (wo)
 * 0x0100 → 0x18FF  video memory
 *
 * COLOR PALETTE
 * The 48 first bytes are used for the palette and must be set by the software
 * at startup. They define 16 colors using 3 bytes each, 1 for each RGB color.
 *
 * KEYBOARD REGISTER
 * The key byte is updated by the minifb library every time the video memory is
 * refreshed. Write has no effect on this register.
 *
 * SCROLLING REGISTER
 * When a value is written in this register, the whole memory video is left
 * shifted by as a many bytes. As each line is 64 byte wide is is possible to
 * add 4 lines at the bottom of the screen. All added lines are filled with
 * bytes 0x00.
 *
 * VIDEO MEMORY
 * Each byte of the video memory is mapped to 2 pixels in the framebuffer
 * memory, 4 bits defining the index in the palette for the RGB values for each
 * pixel.
 *
 * Performances are really low for now, even compared to real world 6502 based
 * hardware. Who cares?
 */

use super::*;
use minifb::{InputCallback, Window};
use std::sync::mpsc;

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

pub struct MiniFBMemoryAdapter {
    palette: [u8; 48],
    minifb: Vec<u32>,
    memory: Vec<u8>,
    window: Window,
    receiver: mpsc::Receiver<u32>,
    last_key: u8,
}

impl MiniFBMemoryAdapter {
    pub fn new(mut window: Window) -> MiniFBMemoryAdapter {
        let (tx, rx) = mpsc::channel::<u32>();
        window.set_input_callback(Box::new(KeyboardBuffer { sender: tx }));

        MiniFBMemoryAdapter {
            palette: [0x00; 48],
            minifb: vec![0; MINIFB_WIDTH * MINIFB_HEIGHT],
            memory: vec![0; MINIFB_WIDTH * MINIFB_HEIGHT / 2],
            window: window,
            receiver: rx,
            last_key: 0x00,
        }
    }

    fn update_minifb_pixel(&mut self, addr: usize) {
        let byte = self.memory[addr];
        let minifb_addr = addr * 2;
        let lo_byte = byte & 0b00001111;
        let hi_byte = byte >> 4;
        let (r, g, b) = (
            self.palette[lo_byte as usize * 3],
            self.palette[lo_byte as usize * 3 + 1],
            self.palette[lo_byte as usize * 3 + 2],
        );
        self.minifb[minifb_addr] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        let (r, g, b) = (
            self.palette[hi_byte as usize * 3],
            self.palette[hi_byte as usize * 3 + 1],
            self.palette[hi_byte as usize * 3 + 2],
        );
        self.minifb[minifb_addr + 1] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
    }

    /*
     * ultra violence
     */
    fn update_minifb(&mut self) {
        for i in 0..self.memory.len() {
            self.update_minifb_pixel(i);
        }
    }
}

impl AddressableIO for MiniFBMemoryAdapter {
    fn get_size(&self) -> usize {
        MINIFB_WIDTH * MINIFB_HEIGHT / 2 + 0xFF
    }

    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if len > 1 {
            panic!("The video driver does not support reading more than a byte at a time for now.");
        }

        // reading the keyboard memory
        if addr == KEY_STACK_POINTER {
            return Ok(vec![self.last_key]);
        } else if addr > 0xff {
            let mem_addr = addr - 0xff;
            return Ok(vec![self.memory[mem_addr]]);
        }

        Ok(vec![0x00])
    }

    fn write(&mut self, addr: usize, data: &Vec<u8>) -> Result<(), MemoryError> {
        if addr < 0x30 {
            for (offset, val) in data.iter().enumerate() {
                self.palette[addr + offset] = *val;
            }
        } else {
            if data.len() > 1 {
                panic!(
                    "writing more than 1 byte at the time is not yet supported by the video driver"
                );
            }
            if addr == 0x31 {
                for _ in 0..data[0] {
                    self.memory.remove(0);
                }
                self.memory.resize(MINIFB_HEIGHT * MINIFB_WIDTH / 2, 0x00);
                self.update_minifb();
            } else if addr >= 0x0100 {
                self.memory[addr - 0x100] = data[0];
                self.update_minifb_pixel(addr - 0x100);
            }
        }
        Ok(())
    }

    fn refresh(&mut self) {
        self.window
            .update_with_buffer(&(self.minifb), MINIFB_WIDTH, MINIFB_HEIGHT)
            .unwrap();
        self.last_key = match self.receiver.try_recv() {
            Ok(v) => u32::to_ne_bytes(v)[0],
            Err(_) => 0x00,
        };
    }
}
