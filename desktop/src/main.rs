use std::fs;

pub mod sdl_frontend;

use rsx::cpu::CPU;
use sdl2::audio::AudioSpecDesired;
use sdl_frontend::{SdlFrontend, PsxAudioCallback};

extern crate rsx;

pub fn main() {
  // let args: Vec<String> = env::args().collect();

  // if args.len() != 2 {
  //   panic!("please specify a file");
  // }

  // let filepath = &args[1];

  // let bytes: Vec<u8> = fs::read(filepath).unwrap();

  let sdl_context = sdl2::init().unwrap();

  let mut cpu = CPU::new(fs::read("../SCPH1001.BIN").unwrap());
  let mut frontend = SdlFrontend::new(&sdl_context);

  let audio_subsystem = sdl_context.audio().unwrap();

  let spec = AudioSpecDesired {
    freq: Some(44100),
    channels: Some(2),
    samples: Some(8192)
  };

  let device = audio_subsystem.open_playback(
    None,
    &spec,
    |_| PsxAudioCallback { spu: &mut cpu.bus.spu }
  ).unwrap();

  device.resume();

  loop {
    while !cpu.bus.gpu.frame_complete {
      cpu.step();
    }

    cpu.bus.gpu.frame_complete = false;

    frontend.render(&mut cpu.bus.gpu);
    frontend.handle_events();
  }
}