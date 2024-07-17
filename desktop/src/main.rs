use std::{collections::VecDeque, env, fs::{self, File}, sync::{Arc, Mutex}};

pub mod sdl_frontend;

use rsx::cpu::CPU;
use sdl_frontend::SdlFrontend;

extern crate rsx;

pub fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    panic!("please specify a path to a game.");
  }

  let audio_samples = Arc::new(Mutex::new(VecDeque::new()));

  let filepath = &args[1];

  let sdl_context = sdl2::init().unwrap();

  let mut cpu = CPU::new(
    fs::read("../SCPH1001.BIN").unwrap(),
    Some(File::open(filepath).unwrap()),
    None,
    false,
    audio_samples.clone()
  );

  let mut frontend = SdlFrontend::new(&sdl_context, audio_samples);

  if args.len() == 3 {
    let exe_file = &args[2];
    cpu.exe_file = Some(exe_file.clone());
  }
  loop {
    cpu.run_frame();
    cpu.bus.gpu.cap_fps();

    cpu.bus.reset_cycles();

    frontend.render(&mut cpu.bus.gpu);
    frontend.handle_events(&mut cpu);
  }
}