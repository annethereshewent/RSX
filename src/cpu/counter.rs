#[derive(Clone, Copy)]
pub enum Device {
  Gpu = 0,
  Dma = 1
}

pub struct Counter {
  pub cycles: i32,
  pub device_sync: [i32; 2],
  pub previous: i32
}

impl Counter {
  pub fn new() -> Self {
    Self {
      cycles: 0,
      previous: 0,
      device_sync: [0; 2]
    }
  }

  pub fn tick(&mut self, cycles: i32) {
    self.cycles += cycles;
  }

  pub fn elapsed(&mut self) -> i32 {
    let elapsed = self.cycles - self.previous;

    self.previous = self.cycles;

    elapsed
  }

  pub fn sync_and_get_elapsed_cycles(&mut self, device: Device) -> i32 {
    let elapsed = self.cycles - self.device_sync[device as usize];

    self.device_sync[device as usize] += elapsed;

    elapsed as i32
  }
}