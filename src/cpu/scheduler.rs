pub struct Scheduler {
  pub cycles: i32
}

impl Scheduler {
  pub fn new() -> Self {
    Self {
      cycles: 0
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    self.cycles += cycles;
  }

  pub fn synchronize_counters(&mut self) {
    self.cycles = 0;
  }
}