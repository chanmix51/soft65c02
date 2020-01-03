use minifb::{Key, Window, WindowOptions, Error, ScaleMode, Scale};
use soft65c02::{Memory, Registers, AddressableIO, LogLine};
use soft65c02::memory::{MINIFB_HEIGHT, MINIFB_WIDTH, MiniFBMemoryAdapter};

#[test]
#[ignore]
fn minifb() {
    use std::{thread, time};
    use std::io::prelude::*;
    use std::fs;

    let init_vector:usize = 0x1B00;
    let mut memory = Memory::new_with_ram();
    let mut window = init_window();
    memory.add_subsystem("VIDEO TERMINAL", 0x0200, MiniFBMemoryAdapter::new(window));
    // ↓ init the video palette
    memory.write(0x0200, vec![
        0x00, 0x00, 0x00, // black
        0x88, 0x00, 0x00, // red
        0x00, 0x88, 0x00, // green
        0x00, 0x00, 0x88, // blue
        0x88, 0x88, 0x00, // yellow
        0x88, 0x00, 0x88, // pink
        0x00, 0x88, 0x88, // cyan
        0x88, 0x88, 0x88, // white
        0x22, 0x22, 0x22, // grey
        0xff, 0x00, 0x00, // intense red
        0x00, 0xff, 0x00, // intense green
        0x00, 0x00, 0xff, // intense blue
        0xff, 0xff, 0x00, // intense yellow
        0xff, 0x00, 0xff, // intense pink
        0x00, 0xff, 0xff, // intense cyan
        0xff, 0xff, 0xff, // intense white
    ]).unwrap();
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
            "65C02 computer graphic test",
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
   adc $0330
   sta $0300,X
   sta $0400,X
   sta $0500,X
   inx
   bne loop
   brk
   */
    vec![
        0xa9, 0x0f, 0x8d, 0x00, 0x80, 0xa9, 0x00, 0xaa,
        0x1a, 0xed, 0x00, 0x80, 0x6d, 0x30, 0x03, 0x9d,
        0x00, 0x03, 0x9d, 0x00, 0x04, 0x9d, 0x00, 0x05,
        0xe8, 0xd0, 0xed, 0x00
    ]
}
