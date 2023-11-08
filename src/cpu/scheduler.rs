use crate::gpu::GPU_FREQUENCY;
use super::CPU_FREQUENCY;


#[derive(Clone, Copy)]
pub enum Schedulable {
  Gpu = 0,
  Dma = 1
}

pub struct Scheduler {
  pub cycles: i64,
  pub device_sync: [i64; 2],
  pub previous: i64
}

const INITIAL_GPU_CYCLES: i32 = (3212.0 * (CPU_FREQUENCY / GPU_FREQUENCY)) as i32;

impl Scheduler {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      previous: 0,
      device_sync: [0; 2]
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    self.cycles += cycles as i64;
  }

  pub fn elapsed(&mut self) -> i32 {
    let elapsed = (self.cycles - self.previous) as i32;

    self.previous = self.cycles;

    elapsed
  }

  pub fn sync_and_get_elapsed_cycles(&mut self, schedulable: Schedulable) -> i32 {
    let elapsed = self.cycles - self.device_sync[schedulable as usize];

    if matches!(schedulable, Schedulable::Dma) {
      println!("the last sync for DMA was {}", self.device_sync[schedulable as usize]);
    }

    self.device_sync[schedulable as usize] += elapsed;

    elapsed as i32
  }
}