use minifb::{Key, Window, WindowOptions, Error, ScaleMode, Scale};
use soft65c02::{Memory, Registers, AddressableIO, LogLine};
use soft65c02::memory::{MINIFB_HEIGHT, MINIFB_WIDTH, MiniFBMemoryAdapter};

#[test]
fn minifb() {
    use std::{thread, time};
    use std::io::prelude::*;
    use std::fs;

    let init_vector:usize = 0x1A00;
    let mut memory = Memory::new_with_ram();
    let mut window = init_window();
    memory.add_subsystem("VIDEO TERMINAL", 0x0200, MiniFBMemoryAdapter::new(window));
    //memory.write(init_vector, vec![0xa9, 0x01, 0x8d, 0x00, 0x02, 0xa9, 0x05, 0x8d, 0x01, 0x02, 0xa9, 0x08, 0x8d, 0x02, 0x02]);
    memory.write(init_vector, dump_program());
    let mut registers = Registers::new(init_vector);
    let mut cp = 0x0000;
    let mut f = fs::File::create("log.txt").unwrap();

    while cp != registers.command_pointer {
        cp = registers.command_pointer;
        writeln!(f, "{}", soft65c02::execute_step(&mut registers, &mut memory).unwrap());
        thread::sleep(time::Duration::from_millis(1));
    }
}

fn init_window() -> Window {
    let mut window = Window::new(
            "Test - ESC to exit",
            MINIFB_WIDTH,
            MINIFB_HEIGHT,
            WindowOptions {
            resize: true,
            scale: Scale::X2,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .unwrap_or_else(|e| { panic!("{}", e); });

    // Limit to max ~60 fps update rate
    window.limit_update_rate(Some(std::time::Duration::from_micros(16600)));

    window
}
fn dump_program() -> Vec<u8> {
   /*
   lda #$0f
   sta $8000
   lda #$00
   tax
loop:
   ina
   sbc $8000
   sta $0200,X
   sta $0300,X
   sta $0400,X
   inx
   bne loop
   brk
   */
    vec![
        0xa9, 0x0f, 0x8d, 0x00, 0x80, 0xa9, 0x00, 0xaa,
        0x1a, 0xed, 0x00, 0x80, 0x9d, 0x00, 0x02, 0x9d,
        0x00, 0x03, 0x9d, 0x00, 0x04, 0xe8, 0xd0, 0xf0,
        0x00
    ]
}
