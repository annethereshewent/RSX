use std::fs;

pub mod sdl_frontend;

use rustation::cpu::{CPU, CYCLES_PER_FRAME, scheduler::Schedulable};
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

  let mut inner_cycles = 0;
  let mut outer_cycles: i64 = 0;

  loop {
    outer_cycles = 0;
    while outer_cycles < CYCLES_PER_FRAME {
      inner_cycles = 0;
      while inner_cycles < 128 {
        cpu.step();

        let elapsed = cpu.bus.scheduler.elapsed();

        inner_cycles += elapsed;
        outer_cycles += elapsed as i64;
      }
      cpu.bus.tick_all();
    }

    frontend.handle_events();
  }
}