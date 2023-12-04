use std::{fs::{self, File}, env};

pub mod sdl_frontend;

use rsx::cpu::CPU;
use sdl_frontend::SdlFrontend;

extern crate rsx;

pub fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    panic!("please specify a file");
  }

  let filepath = &args[1];

  let game_file = File::open(filepath).unwrap();

  let sdl_context = sdl2::init().unwrap();

  let mut cpu = CPU::new(fs::read("../SCPH1001.BIN").unwrap(), game_file);

  let mut frontend = SdlFrontend::new(&sdl_context);

  if args.len() == 3 {
    cpu.load_exe(&args[2]);
  }

  loop {
    while !cpu.bus.gpu.frame_complete {
      cpu.step();
    }

    cpu.bus.gpu.frame_complete = false;

    frontend.render(&mut cpu.bus.gpu);
    frontend.handle_events(&mut cpu);
    frontend.push_samples(cpu.bus.spu.audio_buffer.drain(..).collect());
  }
}