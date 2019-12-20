#[derive(Debug)]
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
