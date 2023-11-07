use crate::gpu::GPU_FREQUENCY;

use super::{CPU, CPU_FREQUENCY};

pub struct Scheduler {
  pub cycles: i32,
  pub upcoming_event: i32,
  pub last_sync: i32
}

const INITIAL_CYCLES: i32 = (3212.0 * (CPU_FREQUENCY / GPU_FREQUENCY)) as i32;

impl Scheduler {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      // this is initially set to the first gpu event for now
      upcoming_event: INITIAL_CYCLES,
      last_sync: 0
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    self.cycles += cycles;
  }

  pub fn synchronize_counters(&mut self) {
    self.upcoming_event -= self.cycles;
    self.last_sync -= self.cycles;

    self.cycles = 0;
  }

  pub fn has_pending_events(&self) -> bool {
    self.cycles >= self.upcoming_event
  }

  pub fn get_elapsed_cycles(&mut self) -> i32 {
    let elapsed = self.cycles - self.last_sync;

    self.last_sync = self.cycles;

    elapsed
  }

  pub fn schedule_next_event(&mut self, delta: i32) {
    self.upcoming_event = self.cycles + delta;
  }
}