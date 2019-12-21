use std::fmt;

pub struct Registers {
    pub accumulator:        u8,
    pub register_x:         u8,
    pub register_y:         u8,
    pub status_register:    u8,
    pub command_pointer:    usize,
    pub stack_pointer:      u8,
}

impl Registers {
    pub fn new(init_address: usize) -> Registers {
        Registers {
            accumulator:        0x00,
            register_x:         0x00,
            register_y:         0x00,
            status_register:    0b00110000,
            command_pointer:    init_address,
            stack_pointer:      0xff,
        }
    }
}

impl fmt::Debug for Registers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
        f,
        "Registers {{ A: {:02x}, X: {:02x}, Y: {:02x} | SP: {:02x} CP: {:04x}\rV-BDIZC\r{:08b} }}",
        self.accumulator,
        self.register_x,
        self.register_y,
        self.stack_pointer,
        self.command_pointer,
        self.status_register
        )
    }
}
