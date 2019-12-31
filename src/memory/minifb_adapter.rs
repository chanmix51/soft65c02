use minifb::Window;
use super::*;

pub const MINIFB_WIDTH: usize = 128;
pub const MINIFB_HEIGHT: usize = 96;

pub struct MiniFBMemoryAdapter {
    minifb: Vec<u32>,
    palette: [(u8, u8, u8); 16],
    memory: Box<[u8; MINIFB_WIDTH * MINIFB_HEIGHT / 2]>
}

impl MiniFBMemoryAdapter {
    pub fn new() -> MiniFBMemoryAdapter {
        MiniFBMemoryAdapter {
            minifb: vec![0; MINIFB_WIDTH * MINIFB_HEIGHT],
            palette: [
                (0x00, 0x00, 0x00), // black
                (0x88, 0x00, 0x00), // red
                (0x00, 0x88, 0x00), // green
                (0x00, 0x00, 0x88), // blue
                (0x88, 0x88, 0x00), // yellow
                (0x88, 0x00, 0x88), // pink
                (0x00, 0x88, 0x88), // cyan
                (0x88, 0x88, 0x88), // white
                (0x22, 0x22, 0x22), // grey
                (0xff, 0x00, 0x00), // intense red
                (0x00, 0xff, 0x00), // intense green
                (0x00, 0x00, 0xff), // intense blue
                (0xff, 0xff, 0x00), // intense yellow
                (0xff, 0x00, 0xff), // intense pink
                (0x00, 0xff, 0xff), // intense cyan
                (0xff, 0xff, 0xff), // intense white
                ],
            memory: Box::new([0; MINIFB_WIDTH * MINIFB_HEIGHT / 2]),
        }
    }

    fn update_minifb(&mut self, addr: usize) {
        let minifb_addr = addr * 2;
        let byte = self.memory[addr];
        let lo_byte = byte & 0b00001111;
        let hi_byte = byte >> 4;
        let (r, g, b) = self.palette[lo_byte as usize];
        self.minifb[minifb_addr] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
        let (r, g, b) = self.palette[hi_byte as usize];
        self.minifb[minifb_addr + 1] = ((r as u32) << 16) | ((g as u32) << 8) | b as u32;
    }

    pub fn window_update(&self, window: &mut Window) -> Result<(), minifb::Error> {
        window.update_with_buffer(&(self.minifb), MINIFB_WIDTH, MINIFB_HEIGHT)
    }
}

impl AddressableIO for MiniFBMemoryAdapter {
    fn get_size(&self) -> usize {
        MINIFB_WIDTH * MINIFB_HEIGHT / 2
    }

    fn read(&self, addr: usize, len: usize) -> Result<Vec<u8>, MemoryError> {
        if addr + len >= self.get_size() {
            return Err(MemoryError::ReadOverflow(len, addr, self.get_size()));
        }

        let output = self.memory[addr..addr + len].to_vec();

        Ok(output)
    }

    /*
     * TODO: handle overflows
     */
    fn write(&mut self, addr: usize, data: Vec<u8>) -> Result<(), MemoryError> {
        let mut offset = 0;
        for byte in data.iter() {
            let pointer = addr + offset;
            self.memory[pointer] = *byte;
            self.update_minifb(pointer);
            offset += 1;
        }

        Ok(())
    }
}
