use std::fs;

pub mod sdl_frontend;

use rustation::cpu::CPU;
use sdl_frontend::SdlFrontend;

extern crate rustation;

pub fn main() {
  // let args: Vec<String> = env::args().collect();

  // if args.len() != 2 {
  //   panic!("please specify a file");
  // }

  // let filepath = &args[1];

  // let bytes: Vec<u8> = fs::read(filepath).unwrap();

  let mut cpu = CPU::new(fs::read("../SCPH1001.BIN").unwrap());
  let mut frontend = SdlFrontend::new();

  loop {
    for _ in 0..1_000_000 {
      cpu.step();
    }

    frontend.handle_events();
  }
}