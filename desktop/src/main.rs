use std::{env, fs::{self, File}, path::Path};

pub mod sdl_frontend;

use rsx::cpu::CPU;
use sdl_frontend::SdlFrontend;

extern crate rsx;

pub fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    panic!("please specify a path to a game or PS exe.");
  }

  let filepath = Path::new(&args[1]);

  let sdl_context = sdl2::init().unwrap();

  let file_extension = filepath.extension().unwrap_or_default().to_str().unwrap_or_default();

  let bios_data = fs::read("../SCPH1001.BIN").unwrap();

  let mut cpu = if file_extension == "exe" {
    let mut cpu = CPU::new(bios_data, None, None, false);
    cpu.exe_file = Some(args[1].to_string());

    cpu
  } else {
    CPU::new(bios_data, Some(File::open(filepath).unwrap()), None, false)
  };

  let mut frontend = SdlFrontend::new(&sdl_context);

  loop {
    cpu.run_frame();
    cpu.bus.gpu.cap_fps();

    cpu.bus.reset_cycles();

    frontend.render(&mut cpu.bus.gpu);
    frontend.handle_events(&mut cpu);
    frontend.push_samples(cpu.bus.spu.audio_buffer.drain(..).collect());
  }
}