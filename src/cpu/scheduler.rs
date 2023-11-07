use crate::gpu::GPU_FREQUENCY;
use super::{CPU_FREQUENCY, dma::DMA_CYCLES};


#[derive(Clone, Copy)]
pub enum Schedulable {
  Gpu = 0,
  Dma = 1
}

pub struct Scheduler {
  pub cycles: i32,
  pub upcoming_event: i32,
  pub upcoming_events: [i32; 2],
  pub last_sync: [i32; 2]
}

const INITIAL_GPU_CYCLES: i32 = (3212.0 * (CPU_FREQUENCY / GPU_FREQUENCY)) as i32;

impl Scheduler {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      // this is initially set to the first gpu event for now
      upcoming_event: DMA_CYCLES,
      last_sync: [0; 2],
      upcoming_events: [
        INITIAL_GPU_CYCLES,
        DMA_CYCLES
      ]
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    self.cycles += cycles;
  }

  pub fn synchronize_counters(&mut self) {
    self.upcoming_event -= self.cycles;

    for i in 0..2 {
      self.last_sync[i] -= self.cycles;
      self.upcoming_events[i] -= self.cycles;
    }

    self.cycles = 0;
  }

  pub fn has_pending_events(&self) -> bool {
    self.cycles >= self.upcoming_event
  }

  pub fn get_elapsed_cycles(&mut self, schedulable: Schedulable) -> i32 {
    let elapsed = self.cycles - self.last_sync[schedulable as usize];

    self.last_sync[schedulable as usize] = self.cycles;

    elapsed
  }

  pub fn schedule_next_event(&mut self, delta: i32, schedulable: Schedulable) {
    self.upcoming_events[schedulable as usize] = self.cycles + delta;

    self.update_upcoming_event();
  }

  pub fn update_upcoming_event(&mut self) {
    self.upcoming_event = *self.upcoming_events.iter().min().unwrap();
  }
}