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

  loop {
    while cpu.bus.scheduler.cycles < CYCLES_PER_FRAME {
      while !cpu.bus.scheduler.has_pending_events() {
        cpu.step();
      }

      while cpu.bus.scheduler.has_pending_events() {
        if cpu.bus.scheduler.upcoming_event >= cpu.bus.scheduler.upcoming_events[Schedulable::Gpu as usize] {
          cpu.bus.gpu.step(&mut cpu.bus.scheduler);
        } if cpu.bus.scheduler.upcoming_event >= cpu.bus.scheduler.upcoming_events[Schedulable::Dma as usize] {
          cpu.bus.dma.step(&mut cpu.bus.scheduler);
        }
      }
    }

    frontend.handle_events();
    cpu.bus.scheduler.synchronize_counters();
  }
}