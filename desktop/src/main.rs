use std::{fs::{self, File}, env};

pub mod sdl_frontend;

use rsx::cpu::CPU;
use sdl_frontend::SdlFrontend;

extern crate rsx;

pub fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    panic!("please specify a path to a game.");
  }

  let filepath = &args[1];

  let game_file = File::open(filepath).unwrap();

  let sdl_context = sdl2::init().unwrap();

  let mut cpu = CPU::new(fs::read("../SCPH1001.BIN").unwrap(), game_file);

  let mut frontend = SdlFrontend::new(&sdl_context);

  if args.len() == 3 {
    let exe_file = &args[2];
    cpu.exe_file = Some(exe_file.clone());
  }
  loop {
    while !cpu.bus.gpu.frame_complete {
      while cpu.bus.cycles - cpu.bus.last_sync < 128 {
        cpu.step();
      }

      cpu.bus.sync_devices();
    }

    cpu.bus.gpu.frame_complete = false;

    cpu.bus.reset_cycles();

    frontend.render(&mut cpu.bus.gpu);
    frontend.handle_events(&mut cpu);
    frontend.push_samples(cpu.bus.spu.audio_buffer.drain(..).collect());
  }
}